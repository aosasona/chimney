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

macro_rules! handle_redirect {
    ($server:expr, $request_path:expr) => {
        if let Some((to, replay)) = $server.find_redirect($request_path) {
            return Ok(redirect(to, replay));
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
                        match res {
                            Ok((stream, _)) => {
                                let io = TokioIo::new(stream);
                                let service = service_fn(|req| serve_file(&self_clone, req));
                                let conn = http1::Builder::new().serve_connection(io, service);

                                if let Err(error) = conn.await {
                                    log_error!(error);
                                }
                            }
                            Err(error) => {
                                log_error!(error);
                            }
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
    handle_redirect!(server, request_path);

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
            // The rewrite may be pointing to a redirect even if it is not a valid file, so we need to check
            // for that here
            handle_redirect!(server, target);
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
