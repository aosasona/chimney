use crate::{
    config::{Config, Rewrite},
    error::ChimneyError::{self, FailedToAcceptConnection, FailedToBind, FailedToParseAddress},
    log_request,
    server::tokio_rt::TokioIo,
};
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::{server::conn::http1, service::service_fn};
use hyper::{Request, Response, Result as HyperResult};
use std::net::SocketAddr;
use tokio::net::TcpListener;

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
    // HTTPS)
    async fn listen(self) -> Result<(), ChimneyError> {
        let raw_addr = format!("{}:{}", self.config.host, self.config.port);
        let addr: SocketAddr = raw_addr
            .parse()
            .map_err(|e| FailedToParseAddress(raw_addr, e))?;

        let server = TcpListener::bind(addr).await.map_err(|e| FailedToBind(e))?;

        println!(
            "\x1b[92mServer is running at http://{}:{}\x1b[0m",
            self.config.host, self.config.port
        );

        loop {
            let (stream, _) = server
                .accept()
                .await
                .map_err(|e| FailedToAcceptConnection(e))?;

            let self_clone = self.clone();

            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let service = service_fn(|req| serve_file(&self_clone, req));
                if let Err(error) = http1::Builder::new().serve_connection(io, service).await {
                    eprintln!("\x1b[91m[Error] {:?}\x1b[0m", error);
                }
            });
        }
    }
}

async fn serve_file(
    server: &Server,
    req: Request<hyper::body::Incoming>,
) -> HyperResult<Response<BoxBody<Bytes, std::io::Error>>> {
    let mut target = req.uri().path();

    if server.config.enable_logging {
        log_request!(&req);
    }

    // TODO: check for the existence of a rewrite rule for the requested path
    if server.config.rewrites.len() > 0 {
        let rewrite_key = if target.starts_with("/") {
            target.to_string()
        } else {
            format!("/{}", target)
        };

        assert!(rewrite_key.starts_with("/"));

        if let Some(rewrite) = server.config.rewrites.get(&rewrite_key) {
            target = match rewrite {
                Rewrite::Config { to } => to,
                Rewrite::Target(target) => target,
            };
        };
    }

    // TODO: check for the existence of a file at the requested path

    todo!()
}
