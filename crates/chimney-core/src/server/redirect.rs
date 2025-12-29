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

    /// Build a redirect response using the resolved host
    fn build_redirect_response(req: &Request<Incoming>, host: &str) -> Response<Full<Bytes>> {
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
        let inner = self.inner.clone();
        let config_handle = self.config_handle.clone();
        let is_https = self.is_https;

        Box::pin(async move {
            // Only redirect if this is an HTTP request (not HTTPS)
            if is_https {
                return inner.call(req).await;
            }

            // Resolve the host using the configured strategy
            let resolved = match inner.resolve_host(req.headers()).await {
                Ok(resolved) => resolved,
                Err(_) => {
                    // If we can't resolve the host, just pass through to inner service
                    return inner.call(req).await;
                }
            };

            // Check if global HTTPS is enabled and site has auto_redirect enabled
            let config = config_handle.get();

            // Global HTTPS must be enabled
            let global_https_enabled = config
                .https
                .as_ref()
                .map(|https| https.enabled)
                .unwrap_or(false);

            if !global_https_enabled {
                return inner.call(req).await;
            }

            // Check site-specific auto_redirect (defaults to true)
            let should_redirect = if let Some(site) = config.sites.find_by_hostname(&resolved.host) {
                site.https_config
                    .as_ref()
                    .map(|https| https.auto_redirect)
                    .unwrap_or(true) // Default to true when no site-specific config
            } else {
                // Site not found, don't redirect
                false
            };

            if should_redirect {
                let response = Self::build_redirect_response(&req, &resolved.host);
                Ok(response)
            } else {
                inner.call(req).await
            }
        })
    }
}
