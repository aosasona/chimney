use crate::{
    config::{Config, Mode, Redirect, Rewrite},
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
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};
use tokio::{fs::File, net::TcpListener, sync::Notify};
use tokio_util::io::ReaderStream;

#[derive(Debug, Clone)]
pub struct Server {
    host: IpAddr,
    port: usize,
    mode: Mode,
    pub enable_logging: bool,
    root_dir: PathBuf,
    ignore_matches: Vec<String>,
    pub sites: HashMap<String, Config>,
    // Key: domain without protocol (e.g thing.foo.bar), value: site
    // This is so that we can easily jump from an host to a config in the `sites` "table" without
    // any sort of traversal or looping
    pub domain_mappings: HashMap<String, String>,
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
    ($server:expr, $config:expr, $request_path:expr) => {
        if let Some((to, replay)) = $server.find_redirect($config, $request_path) {
            return Ok(redirect(to, replay));
        }
    };
}

pub struct Opts {
    pub host: IpAddr,
    pub port: usize,
    pub mode: Mode,
    pub enable_logging: bool,
    pub root_dir: PathBuf,
    pub ignore_matches: Vec<String>,
}

impl Server {
    pub fn new(opts: &Opts) -> Self {
        Server {
            host: opts.host,
            port: opts.port,
            enable_logging: opts.enable_logging,
            mode: opts.mode.clone(),
            root_dir: opts.root_dir.clone(),
            ignore_matches: opts.ignore_matches,
            sites: HashMap::new(),
            domain_mappings: HashMap::new(),
            shutdown_signal: Arc::new(Notify::new()),
        }
    }

    pub fn set_host(&mut self, host: IpAddr) -> &Self {
        self.host = host;
        return self;
    }

    pub fn set_port(&mut self, port: usize) -> &Self {
        self.port = port;
        return self;
    }

    pub fn set_enable_logging(&mut self, enable_logging: bool) -> &Self {
        self.enable_logging = enable_logging;
        return self;
    }

    pub fn set_root_dir(&mut self, root_dir: PathBuf) -> &Self {
        self.root_dir = root_dir;
        return self;
    }

    pub fn set_mode(&mut self, mode: Mode) -> &Self {
        self.mode = mode;
        return self;
    }

    /// Add a new site and its config to the server's source of truth
    pub fn register(&mut self, site_name: String, config: &Config) -> &Self {
        if self.sites.get(&site_name).is_some() {
            return self;
        }

        // TODO: battle the lifetime and stop cloning
        self.sites.insert(site_name, config.clone());
        return self;
    }

    pub fn find_config_by_host<'a>(&'a self, host: &'a str) -> Option<&'a Config> {
        return match self.mode {
            Mode::Single => self.sites.get("default"),
            Mode::Multi => {
                let site_name = self.domain_mappings.get(host)?;
                self.sites.get(site_name)
            }
        };
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

    async fn listen(self) -> Result<(), ChimneyError> {
        let raw_addr = format!("{}:{}", self.host, self.port);
        let addr: SocketAddr = raw_addr
            .parse()
            .map_err(|e| FailedToParseAddress(raw_addr, e))?;

        let server = TcpListener::bind(addr).await.map_err(FailedToBind)?;

        log_info!(format!(
            "Server is running at http://{}:{}",
            self.host, self.port
        ));

        let arc_self = Arc::new(self.clone());

        loop {
            let self_clone = Arc::clone(&arc_self);

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

    pub fn find_rewrite_or(&self, config: &Config, target: &str) -> String {
        if config.rewrites.is_empty() {
            return target.to_string();
        }

        let rewrite_key = with_leading_slash!(target);
        assert!(rewrite_key.starts_with('/'));

        if let Some(rewrite) = config.rewrites.get(&rewrite_key) {
            return with_leading_slash!(match rewrite {
                Rewrite::Config { to } => to,
                Rewrite::Target(target) => target,
            })
            .to_string();
        };

        with_leading_slash!(target)
    }

    pub fn find_redirect(&self, config: &Config, path: &str) -> Option<(String, bool)> {
        if config.redirects.is_empty() {
            return None;
        }

        let redirect_key = with_leading_slash!(path);
        assert!(redirect_key.starts_with('/'));

        if let Some(redirect) = config.redirects.get(&redirect_key) {
            return match redirect {
                Redirect::Target(to) => Some((to.to_string(), false)),
                Redirect::Config { to, replay } => Some((to.to_string(), *replay)),
            };
        };

        None
    }

    pub fn get_valid_file_path(&self, config: &Config, target: &str) -> Option<PathBuf> {
        let mut path = PathBuf::from(&config.root.get_path()).join(target.trim_start_matches('/'));

        if !path.exists() {
            if let Some(fallback) = config.fallback_document.clone() {
                let fallback_path = PathBuf::from(&config.root.get_path()).join(fallback);
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
        config: &Config,
        boxed_body: BoxBody<Bytes, std::io::Error>,
        mime_type: String,
    ) -> Response<BoxBody<Bytes, std::io::Error>> {
        let mut response = Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime_type)
            .header("Server", "Chimney");

        if let Some(headers) = response.headers_mut() {
            for (key, value) in config.headers.iter() {
                if let Ok(header_name) = HeaderName::from_str(key) {
                    headers.insert(header_name, HeaderValue::from_str(value).unwrap());
                }
            }
        }

        match response.body(boxed_body) {
            Ok(response) => response,
            Err(_) => empty_response(StatusCode::NOT_FOUND),
        }
    }
}

fn empty_response(code: StatusCode) -> Response<BoxBody<Bytes, std::io::Error>> {
    Response::builder()
        .status(code)
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

    if server.enable_logging {
        log_request!(&req);
    }

    let target_host = match req.headers().get("host") {
        Some(header_value) => match header_value.to_str() {
            Ok(host) => host,
            _ => return Ok(empty_response(StatusCode::INTERNAL_SERVER_ERROR)),
        },
        None => match req.uri().host() {
            Some(host) => host,
            _ => return Ok(empty_response(StatusCode::MISDIRECTED_REQUEST)),
        },
    };

    let config = match server.find_config_by_host(target_host) {
        Some(c) => c,
        None => return Ok(empty_response(StatusCode::MISDIRECTED_REQUEST)),
    };

    // Redirects take precedence over rewrites, we need to check for that first before any attempt
    // to normalize the path (with index.html for example) or rewrite it
    handle_redirect!(server, config, request_path);

    // We are not normalizing the path here because we want a rewrite for `/` to be possible
    // assuimg the rewrite is defined in the config file, we don't want to simply overwrite it with
    // `/index.html`
    let target = server.find_rewrite_or(config, request_path);

    // We need to automatically rewrite `/` to `/index.html` if the path is empty since they are
    // generally considered one and the same
    let target = empty_to_index!(target);

    let path = match server.get_valid_file_path(config, &target) {
        Some(path) => path,
        None => {
            // The rewrite may be pointing to a redirect even if it is not a valid file, so we need to check
            // for that here
            handle_redirect!(server, config, target);
            return Ok(empty_response(StatusCode::NOT_FOUND));
        }
    };

    let mime_type = mimetype::from_pathbuf(&path);

    let file: File = match File::open(path).await.map_err(UnableToOpenFile) {
        Ok(file) => file,
        Err(error) => {
            log_error!(format!("Failed to open file: {:?}", error));
            return Ok(empty_response(StatusCode::NOT_FOUND));
        }
    };

    let reader_stream = ReaderStream::new(file);
    let boxed_body = StreamBody::new(reader_stream.map_ok(Frame::data)).boxed();

    let response = server.build_response(config, boxed_body, mime_type);

    Ok(response)
}
