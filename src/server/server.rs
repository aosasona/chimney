use crate::{
    config::{Config, Rewrite},
    error::ChimneyError::{
        self, FailedToAcceptConnection, FailedToBind, FailedToParseAddress, UnableToOpenFile,
    },
    log_error, log_info, log_request,
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
use std::{net::SocketAddr, path::PathBuf, str::FromStr};
use tokio::{fs::File, net::TcpListener};
use tokio_util::io::ReaderStream;

#[derive(Debug, Clone)]
pub struct Server {
    pub config: Config,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Server { config }
    }

    pub async fn run(self) -> Result<(), ChimneyError> {
        self.listen().await?;

        Ok(())
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
            let (stream, _) = server.accept().await.map_err(FailedToAcceptConnection)?;

            let self_clone = self.clone();

            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let service = service_fn(|req| serve_file(&self_clone, req));
                let conn = http1::Builder::new().serve_connection(io, service);

                if let Err(error) = conn.await {
                    log_error!(error);
                }
            });
        }
    }
}

fn not_found() -> Response<BoxBody<Bytes, std::io::Error>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(
            Full::new("Not Found".into())
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap()
}

// TODO: cleanup
async fn serve_file(
    server: &Server,
    req: Request<hyper::body::Incoming>,
) -> HyperResult<Response<BoxBody<Bytes, std::io::Error>>> {
    let mut target = req.uri().path();
    if target.trim_end_matches('/').is_empty() {
        target = "/index.html";
    }

    if server.config.enable_logging {
        log_request!(&req);
    }

    if !server.config.rewrites.is_empty() {
        let rewrite_key = if target.starts_with('/') {
            target.to_string()
        } else {
            format!("/{}", target)
        };

        assert!(rewrite_key.starts_with('/'));

        if let Some(rewrite) = server.config.rewrites.get(&rewrite_key) {
            target = match rewrite {
                Rewrite::Config { to } => to,
                Rewrite::Target(target) => target,
            };
        };
    };

    let mut path = PathBuf::from(&server.config.root_dir).join(target.trim_start_matches('/'));

    if !path.exists() {
        if let Some(fallback) = server.config.fallback_document.clone() {
            let fallback_path = PathBuf::from(&server.config.root_dir).join(fallback);
            if fallback_path.exists() {
                path = fallback_path;
            }
        } else {
            return Ok(not_found());
        }
    };

    // impl: https://github.com/hyperium/hyper/blob/00a703a9ef268266f8a8f78540253cbb2dcc6a55/examples/send_file.rs#L67-L91
    let file = File::open(path).await.map_err(UnableToOpenFile);
    if file.is_err() {
        log_error!(format!("Failed to open file: {:?}", file.err()));
        return Ok(not_found());
    };

    let file: File = file.unwrap();
    let reader_stream = ReaderStream::new(file);

    let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
    let boxed_body = stream_body.boxed();

    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .header("Server", "Chimney")
        .header("X-Content-Type-Options", "nosniff");

    if let Some(headers) = response.headers_mut() {
        for (key, value) in server.config.headers.iter() {
            if let Ok(header_name) = HeaderName::from_str(key) {
                headers.insert(header_name, HeaderValue::from_str(value).unwrap());
            }
        }
    }

    Ok(response.body(boxed_body).unwrap())
}
