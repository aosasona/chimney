// HTTP→HTTPS redirect middleware

use std::{future::Future, net::SocketAddr, pin::Pin, sync::Arc, time::Instant};

use http_body_util::Full;
use hyper::{
    Request, Response, StatusCode,
    body::{Bytes, Incoming},
    header,
    service::Service as HyperService,
};
use log::debug;

use crate::config::ConfigHandle;

use super::{
    hooks::{RequestEvent, RequestHooks, RequestProtocol},
    service::Service,
};

/// Redirect service that wraps the main service and handles HTTP→HTTPS redirects
#[derive(Clone)]
pub struct RedirectService {
    inner: Arc<Service>,
    config_handle: ConfigHandle,
    is_https: bool,
    remote_addr: Option<SocketAddr>,
    hooks: RequestHooks,
}

impl RedirectService {
    /// Create a new redirect service
    pub fn new(inner: Service, config_handle: ConfigHandle, is_https: bool) -> Self {
        Self {
            inner: Arc::new(inner),
            config_handle,
            is_https,
            remote_addr: None,
            hooks: RequestHooks::default(),
        }
    }

    pub(crate) fn new_with_context(
        inner: Service,
        config_handle: ConfigHandle,
        is_https: bool,
        remote_addr: Option<SocketAddr>,
        hooks: RequestHooks,
    ) -> Self {
        Self {
            inner: Arc::new(inner),
            config_handle,
            is_https,
            remote_addr,
            hooks,
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
        let remote_addr = self.remote_addr;
        let hooks = self.hooks.clone();

        Box::pin(async move {
            let started = Instant::now();
            let method = req.method().clone();
            let uri = req.uri().clone();
            let request_headers = req.headers().clone();
            let resolved_host = inner
                .resolve_host(req.headers())
                .await
                .ok()
                .map(|host| host.host);
            let site = resolved_host.as_deref().and_then(|host| {
                config_handle
                    .get()
                    .sites
                    .find_by_hostname(host)
                    .map(|site| site.name.clone())
            });

            // Only redirect if this is an HTTP request (not HTTPS)
            let response = if is_https {
                inner.call(req).await
            } else {
                let config = config_handle.get();
                let global_https_enabled = config
                    .https
                    .as_ref()
                    .map(|https| https.enabled)
                    .unwrap_or(false);
                let should_redirect = global_https_enabled
                    && resolved_host.as_deref().is_some_and(|host| {
                        config.sites.find_by_hostname(host).is_some_and(|site| {
                            site.https_config
                                .as_ref()
                                .map(|https| https.auto_redirect)
                                .unwrap_or(true)
                        })
                    });

                if should_redirect {
                    Ok(Self::build_redirect_response(
                        &req,
                        resolved_host
                            .as_deref()
                            .expect("resolved host checked above"),
                    ))
                } else {
                    inner.call(req).await
                }
            };

            if let Ok(response) = &response {
                hooks.notify(&RequestEvent {
                    method,
                    uri,
                    request_headers,
                    host: resolved_host,
                    site,
                    status: response.status(),
                    response_headers: response.headers().clone(),
                    duration: started.elapsed(),
                    remote_addr,
                    protocol: if is_https {
                        RequestProtocol::Https
                    } else {
                        RequestProtocol::Http
                    },
                });
            }
            response
        })
    }
}
