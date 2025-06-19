use http_body_util::Full;
use hyper::body::Bytes;
use hyper::header::HeaderValue;
use hyper::service::Service as HyperService;
use hyper::{Request, Response, body::Incoming as IncomingBody};
use log::debug;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A service handles an incoming HTTP request and returns a response.
/// It handles resolution of requests to the appropriate filesystem paths and other resources.
#[derive(Clone)]
pub struct Service {
    /// The filesystem abstraction used by the server
    filesystem: Arc<dyn crate::filesystem::Filesystem>,

    /// The configuration for the server
    config: Arc<RwLock<crate::config::Config>>,
}

impl Service {
    pub fn new(
        filesystem: Arc<dyn crate::filesystem::Filesystem>,
        config: Arc<RwLock<crate::config::Config>>,
    ) -> Self {
        debug!("Creating a new Resolver instance");
        Service { filesystem, config }
    }
}

impl HyperService<Request<IncomingBody>> for Service {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        debug!(
            "User-Agent: {}",
            req.headers()
                .get("User-Agent")
                .unwrap_or(&HeaderValue::from_static("Unknown"))
                .to_str()
                .unwrap_or("Unknown")
        );
        todo!()
    }
}
