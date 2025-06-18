use http_body_util::Full;
use hyper::body::Bytes;
use hyper::service::Service;
use hyper::{Request, Response, body::Incoming as IncomingBody};
use log::debug;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A resolver handles the resolution of paths and resources in the Chimney server.
#[derive(Clone)]
pub struct Resolver {
    /// The filesystem abstraction used by the server
    filesystem: Arc<dyn crate::filesystem::Filesystem>,

    /// The configuration for the server
    config: Arc<RwLock<crate::config::Config>>,
}

impl Resolver {
    pub fn new(
        filesystem: Arc<dyn crate::filesystem::Filesystem>,
        config: Arc<RwLock<crate::config::Config>>,
    ) -> Self {
        debug!("Creating a new Resolver instance");
        Resolver { filesystem, config }
    }
}

impl Service<Request<IncomingBody>> for Resolver {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        todo!()
    }
}
