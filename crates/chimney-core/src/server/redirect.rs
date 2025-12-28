// HTTP→HTTPS redirect middleware

use std::{future::Future, pin::Pin, sync::Arc};

use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    header,
    service::Service as HyperService,
    Request, Response, StatusCode,
};
use log::debug;

use crate::config::ConfigHandle;

use super::service::Service;

/// Redirect service that wraps the main service and handles HTTP→HTTPS redirects
#[derive(Clone)]
pub struct RedirectService {
    inner: Arc<Service>,
    config_handle: ConfigHandle,
    is_https: bool,
}

impl RedirectService {
    /// Create a new redirect service
    pub fn new(inner: Service, config_handle: ConfigHandle, is_https: bool) -> Self {
        Self {
            inner: Arc::new(inner),
            config_handle,
            is_https,
        }
    }

    /// Check if the request should be redirected to HTTPS
    fn should_redirect(&self, req: &Request<Incoming>) -> bool {
        // Only redirect if this is an HTTP request (not HTTPS)
        if self.is_https {
            return false;
        }

        // Get the Host header to determine which site this is
        let host = match req.headers().get(header::HOST) {
            Some(host) => match host.to_str() {
                Ok(h) => h,
                Err(_) => return false,
            },
            None => return false,
        };

        // Check if global HTTPS is enabled and site has auto_redirect enabled
        let config = self.config_handle.get();

        // Global HTTPS must be enabled
        let global_https_enabled = config
            .https
            .as_ref()
            .map(|https| https.enabled)
            .unwrap_or(false);

        if !global_https_enabled {
            return false;
        }

        // Check site-specific auto_redirect (defaults to true)
        if let Some(site) = config.sites.find_by_hostname(host) {
            return site
                .https_config
                .as_ref()
                .map(|https| https.auto_redirect)
                .unwrap_or(true); // Default to true when no site-specific config
        }

        // Site not found, don't redirect
        false
    }

    /// Build a redirect response
    fn build_redirect_response(&self, req: &Request<Incoming>) -> Response<Full<Bytes>> {
        let host = req
            .headers()
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("localhost");

        let uri = req.uri();
        let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

        let location = format!("https://{host}{path_and_query}");

        debug!("Redirecting to HTTPS: {location}");

        Response::builder()
            .status(StatusCode::MOVED_PERMANENTLY)
            .header(header::LOCATION, location)
            .body(Full::new(Bytes::from("Redirecting to HTTPS")))
            .unwrap()
    }
}

impl HyperService<Request<Incoming>> for RedirectService {
    type Response = Response<Full<Bytes>>;
    type Error = crate::error::ServerError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        // Check if we should redirect
        if self.should_redirect(&req) {
            let response = self.build_redirect_response(&req);
            return Box::pin(async move { Ok(response) });
        }

        // Otherwise, pass through to the inner service
        let fut = self.inner.call(req);
        Box::pin(fut)
    }
}
