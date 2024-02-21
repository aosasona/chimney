use crate::{
    config::{Config, Redirect, Rewrite},
    error::ChimneyError::{
        self, FailedToAcceptConnection, FailedToBind, FailedToParseAddress, UnableToOpenFile,
    },
    log_error, log_info, log_request, mimetype,
};
use bytes::Bytes;
use futures_util::stream::TryStreamExt;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::{
    body::Frame,
    header::{HeaderName, HeaderValue},
    server::conn::http1,
    service::service_fn,
    Request, Response, Result as HyperResult, StatusCode,
};
use hyper_util::rt::TokioIo;
use std::{net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc};
use tokio::{fs::File, net::TcpListener, sync::Notify};
use tokio_util::io::ReaderStream;

#[derive(Debug, Clone)]
pub struct Server {
    pub config: Config,
    shutdown_signal: Arc<Notify>,
}

macro_rules! with_leading_slash {
    ($path:expr) => {
        if $path.starts_with('/') {
            $path.to_string()
        } else {
            format!("/{}", $path)
        }
    };
}

macro_rules! empty_to_index {
    ($path:expr) => {
        match $path.trim() {
            "/" | "" => "/index.html",
            path => path,
        }
    };
}

impl Server {
    pub fn new(config: Config) -> Self {
        Server {
            config,
            shutdown_signal: Arc::new(Notify::new()),
        }
    }

    pub async fn run(self) -> Result<(), ChimneyError> {
        self.watch_for_shutdown_signal().await;
        self.listen().await?;

        Ok(())
    }

    async fn watch_for_shutdown_signal(&self) {
        let signal = self.shutdown_signal.clone();

        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for shutdown signal");

            signal.notify_one();
        });
    }

    // TODO: handle HTTPS (run a second server for HTTPS, and force redirect from HTTP to
    // HTTPS if enabled)
    async fn listen(self) -> Result<(), ChimneyError> {
        let raw_addr = format!("{}:{}", self.config.host, self.config.port);
        let addr: SocketAddr = raw_addr
            .parse()
            .map_err(|e| FailedToParseAddress(raw_addr, e))?;

        let server = TcpListener::bind(addr).await.map_err(FailedToBind)?;

        log_info!(format!(
            "Server is running at http://{}:{}",
            self.config.host, self.config.port
        ));

        loop {
            let self_clone = self.clone();

            tokio::select! {
                _ = self.shutdown_signal.notified() => {
                    log_info!("Shutting down server");
                    return Ok(());
                },

                res = server.accept() => {
                    if let Err(error) = res {
                        log_error!(error);
                        return Err(FailedToAcceptConnection(error));
                    }

                    tokio::spawn(async move {
                        if let Ok((stream, _)) = res {
                            let io = TokioIo::new(stream);
                            let service = service_fn(|req| serve_file(&self_clone, req));
                            let conn = http1::Builder::new().serve_connection(io, service);

                            if let Err(error) = conn.await {
                                log_error!(error);
                            }
                        } else {
                            log_error!("Failed to accept connection");
                        }
                    });
                }
            }
        }
    }

    pub fn find_rewrite_or(&self, target: &str) -> String {
        if self.config.rewrites.is_empty() {
            return target.to_string();
        }

        let rewrite_key = with_leading_slash!(target);
        assert!(rewrite_key.starts_with('/'));

        if let Some(rewrite) = self.config.rewrites.get(&rewrite_key) {
            return with_leading_slash!(match rewrite {
                Rewrite::Config { to } => to,
                Rewrite::Target(target) => target,
            })
            .to_string();
        };

        with_leading_slash!(target)
    }

    pub fn find_redirect(&self, path: &str) -> Option<(String, bool)> {
        if self.config.redirects.is_empty() {
            return None;
        }

        let redirect_key = with_leading_slash!(path);
        assert!(redirect_key.starts_with('/'));

        if let Some(redirect) = self.config.redirects.get(&redirect_key) {
            return match redirect {
                Redirect::Target(to) => Some((to.to_string(), false)),
                Redirect::Config { to, replay } => Some((to.to_string(), *replay)),
            };
        };

        None
    }

    pub fn get_valid_file_path(&self, target: &str) -> Option<PathBuf> {
        let mut path = PathBuf::from(&self.config.root_dir).join(target.trim_start_matches('/'));

        if !path.exists() {
            if let Some(fallback) = self.config.fallback_document.clone() {
                let fallback_path = PathBuf::from(&self.config.root_dir).join(fallback);
                if fallback_path.exists() && fallback_path.is_file() {
                    path = fallback_path;
                };
            }
        };

        if path.is_dir() {
            let directory_root_file = path.join("index.html");

            if directory_root_file.exists() && directory_root_file.is_file() {
                path = directory_root_file;
            };
        }

        if path.exists() && path.is_file() {
            Some(path)
        } else {
            None
        }
    }

    pub fn build_response(
        &self,
        boxed_body: BoxBody<Bytes, std::io::Error>,
        mime_type: String,
    ) -> Response<BoxBody<Bytes, std::io::Error>> {
        let mut response = Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime_type)
            .header("Server", "Chimney");

        if let Some(headers) = response.headers_mut() {
            for (key, value) in self.config.headers.iter() {
                if let Ok(header_name) = HeaderName::from_str(key) {
                    headers.insert(header_name, HeaderValue::from_str(value).unwrap());
                }
            }
        }

        match response.body(boxed_body) {
            Ok(response) => response,
            Err(_) => not_found(),
        }
    }
}

fn not_found() -> Response<BoxBody<Bytes, std::io::Error>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new("".into()).map_err(|e| match e {}).boxed())
        .unwrap()
}

fn redirect(to: String, replay: bool) -> Response<BoxBody<Bytes, std::io::Error>> {
    let status = if replay {
        StatusCode::PERMANENT_REDIRECT // 308
    } else {
        StatusCode::MOVED_PERMANENTLY // 301
    };

    Response::builder()
        .status(status)
        .header("Location", HeaderValue::from_str(&to).unwrap())
        .body(Full::new("".into()).map_err(|e| match e {}).boxed())
        .unwrap()
}

async fn serve_file(
    server: &Server,
    req: Request<hyper::body::Incoming>,
) -> HyperResult<Response<BoxBody<Bytes, std::io::Error>>> {
    let request_path = req.uri().path();

    if server.config.enable_logging {
        log_request!(&req);
    }

    // Redirects take precedence over rewrites, we need to check for that first before any attempt
    // to normalize the path (with index.html for example) or rewrite it
    if let Some((to, replay)) = server.find_redirect(request_path) {
        return Ok(redirect(to, replay));
    }

    // We are not normalizing the path here because we want a rewrite for `/` to be possible
    // assuimg the rewrite is defined in the config file, we don't want to simply overwrite it with
    // `/index.html`
    let target = server.find_rewrite_or(request_path);

    // We need to automatically rewrite `/` to `/index.html` if the path is empty since they are
    // generally considered one and the same
    let target = empty_to_index!(target);

    let path = match server.get_valid_file_path(&target) {
        Some(path) => path,
        None => {
            return Ok(not_found());
        }
    };

    let mime_type = mimetype::from_pathbuf(&path);

    let file: File = match File::open(path).await.map_err(UnableToOpenFile) {
        Ok(file) => file,
        Err(error) => {
            log_error!(format!("Failed to open file: {:?}", error));
            return Ok(not_found());
        }
    };

    let reader_stream = ReaderStream::new(file);
    let boxed_body = StreamBody::new(reader_stream.map_ok(Frame::data)).boxed();

    let response = server.build_response(boxed_body, mime_type);

    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::config::Redirect;

    pub use super::*;

    fn mock_server() -> Server {
        let rewrites = {
            let mut rewrites = HashMap::new();
            rewrites.insert(
                "/home".to_string(),
                Rewrite::Config {
                    to: "/index.html".to_string(),
                },
            );
            rewrites.insert(
                "/page-2".to_string(),
                Rewrite::Target("another_page.html".to_string()),
            );
            rewrites
        };

        let redirects = {
            let mut redirects: HashMap<String, Redirect> = HashMap::new();
            redirects.insert(
                "/twitch".to_string(),
                Redirect::Target("https://twitch.tv".to_string()),
            );
            redirects.insert(
                "/google".to_string(),
                Redirect::Config {
                    to: "https://google.com".to_string(),
                    replay: true,
                },
            );
            redirects
        };

        let config = Config {
            host: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            port: 80,
            enable_logging: true,
            root_dir: "./examples/basic/public".to_string(),
            fallback_document: Some("fallback.html".to_string()),
            domain_names: vec![],
            https: None,
            headers: HashMap::new(),
            rewrites,
            redirects,
        };

        Server::new(config)
    }

    #[test]
    pub fn find_rewrite_or_test() {
        let mut server = mock_server();

        // This is a valid rewrite so it should return the new path
        assert_eq!(server.find_rewrite_or("/home"), "/index.html".to_string());

        // This is a valid rewrite so it should return the new path
        assert_eq!(
            server.find_rewrite_or("/page-2"),
            "/another_page.html".to_string()
        );

        // This is not a valid rewrite so it should return the original path
        assert_eq!(server.find_rewrite_or("/not-found"), "/not-found");

        // This is not a valid rewrite so it should return the original path
        assert_eq!(server.find_rewrite_or("/"), "/".to_string());

        // Now it is a valid rewrite so it should return the new path
        server.config.rewrites.insert(
            "/".to_string(),
            Rewrite::Config {
                to: "/index_rewrite.html".to_string(),
            },
        );
        assert_eq!(
            server.find_rewrite_or("/"),
            "/index_rewrite.html".to_string()
        );
    }

    #[test]
    pub fn get_file_path_test() {
        let mut server = mock_server();

        // This is a valid file so it should return the path to the file
        assert_eq!(
            server.get_valid_file_path("/index.html"),
            Some(PathBuf::from(format!(
                "{}/index.html",
                server.config.root_dir
            )))
        );

        // the fallback path doesn't exist, the file doesn't exist, and the directory doesn't
        // exist, so we should get back None
        assert_eq!(server.get_valid_file_path("/not-found"), None);

        // this is a valid fallback document so it should return the path to the fallback in this
        // case
        server.config.fallback_document = Some("another_page.html".to_string());
        assert_eq!(
            server.get_valid_file_path("/not-found"),
            Some(PathBuf::from(format!(
                "{}/{}",
                server.config.root_dir,
                server.config.fallback_document.clone().unwrap()
            )))
        );

        // The has no root html file but since it is a directory and it has an index.html file,
        // it should return the path to the index.html file
        server.config.root_dir = "./examples/basic".to_string();
        assert_eq!(
            server.get_valid_file_path("/public"),
            Some(PathBuf::from(format!(
                "{}/public/index.html",
                server.config.root_dir
            )))
        );

        server.config.root_dir = "./examples/trulyao/blog/arguments".to_string();
        assert_eq!(
            server.get_valid_file_path("/"),
            Some(PathBuf::from(format!(
                "{}/index.html",
                server.config.root_dir
            )))
        );

        // This directory has no index.html file so it should return None
        server.config.root_dir = "./examples/trulyao/images".to_string();
        assert_eq!(server.get_valid_file_path("/"), None);
    }

    #[test]
    pub fn find_redirect_test() {
        let mut server = mock_server();

        // This is a valid redirect so it should return the new path
        assert_eq!(
            server.find_redirect("/twitch"),
            Some(("https://twitch.tv".to_string(), false))
        );

        // This is a valid redirect so it should return the new path
        assert_eq!(
            server.find_redirect("/google"),
            Some(("https://google.com".to_string(), true))
        );

        // This is not a valid redirect so it should return None
        assert_eq!(server.find_redirect("/not-found"), None);

        // Test for `/`
        assert_eq!(server.find_redirect("/"), None);

        // Now it is a valid redirect so it should return the new path
        server.config.redirects.insert(
            "/".to_string(),
            Redirect::Target("https://example.com".to_string()),
        );

        assert_eq!(
            server.find_redirect("/"),
            Some(("https://example.com".to_string(), false))
        );
    }
}
