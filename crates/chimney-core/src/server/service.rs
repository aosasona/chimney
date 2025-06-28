use http_body_util::Full;
use hyper::body::Bytes;
use hyper::header::{self, HeaderValue};
use hyper::service::Service as HyperService;
use hyper::{HeaderMap, StatusCode};
use hyper::{Request, Response, body::Incoming as IncomingBody};
use log::{debug, info, trace};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::{HostDetectionStrategy, RedirectRule, RewriteRule};
use crate::error::ServerError;

pub struct DetectedHost {
    /// The detected host, which can be a domain or an IP address
    pub host: String,

    /// Whether the host was detected in auto-detect mode
    pub is_auto: bool,

    /// The header used to detect the host
    pub header: String,
}

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

    /// Resolves the host from the request headers using the cached resolved host header.
    pub async fn resolve_host_with_cache(
        &self,
        headers: &HeaderMap<HeaderValue>,
    ) -> Result<DetectedHost, crate::error::ServerError> {
        let config = self.config.read().await;

        let resolved_header_name = config
            .resolved_host_header()
            .ok_or(crate::error::ServerError::MissingResolvedHostHeader)?;

        debug!(
            "Using cached resolved host header: {}",
            resolved_header_name
        );

        if let Some(value) = headers.get(&resolved_header_name) {
            if let Ok(host) = value.to_str() {
                return Ok(DetectedHost {
                    host: host.to_string(),
                    is_auto: config.host_detection.is_auto(),
                    header: resolved_header_name,
                });
            }

            return Err(crate::error::ServerError::HostDetectionFailed {
                message: format!(
                    "Cached header '{}' is not a valid UTF-8 string",
                    resolved_header_name
                ),
            });
        }

        Err(crate::error::ServerError::HostDetectionFailed {
            message: format!(
                "Cached header '{}' not found in request headers",
                resolved_header_name
            ),
        })
    }

    /// Resolves the host from the request headers using the configured host detection strategy.
    pub async fn resolve_host_with_strategy(
        &self,
        headers: &HeaderMap<HeaderValue>,
    ) -> Result<DetectedHost, crate::error::ServerError> {
        let config = self.config.read().await;
        let target_headers = config.host_detection.target_headers();
        trace!(
            "Using host detection strategy: {:?}, target headers: {:?}",
            config.host_detection, target_headers
        );

        if target_headers.is_empty() {
            debug!(
                "No target headers configured for host detection, current configuration: {:?}",
                config.host_detection
            );
            return Err(crate::error::ServerError::HostDetectionUnspecified);
        }

        // We need to check each header in the order specified by the configuration and return the first one that matches.
        for header in target_headers {
            match headers.get(&header) {
                Some(value) => {
                    if let Ok(host) = value.to_str() {
                        return Ok(DetectedHost {
                            host: host.to_string(),
                            is_auto: config.host_detection.is_auto(),
                            header: header.clone(),
                        });
                    }

                    debug!("Header '{}' is not a valid UTF-8 string header", header);
                }
                None => {
                    debug!("Header '{}' not found in request", header);
                }
            }
        }

        debug!("No valid target host found in request headers");
        Err(crate::error::ServerError::HostDetectionFailed {
            message: "No valid target host found in request headers".to_string(),
        })
    }

    /// Resolves the host from the request headers based on the configured host detection strategy.
    pub async fn resolve_host(
        &self,
        headers: &HeaderMap<HeaderValue>,
    ) -> Result<DetectedHost, crate::error::ServerError> {
        #[cfg(debug_assertions)]
        let start = std::time::Instant::now();

        let config = self.config.read().await;

        // If we have a cached resolved host header, we can use that for our lookup.
        if config.has_resolved_host_header() {
            if let Ok(resolved) = self.resolve_host_with_cache(headers).await {
                #[cfg(debug_assertions)]
                {
                    let elapsed = start.elapsed();
                    debug!(
                        "Resolved host header '{}' in {:?} using cached value",
                        resolved.header, elapsed
                    );
                }

                return Ok(resolved);
            }

            debug!(
                "Cached resolved host header not found in request headers, falling back to configured strategy"
            );
        }

        // At this point, we know we don't have a cached resolved host header, we will proceed with the configured host detection strategy.
        let resolved = self.resolve_host_with_strategy(headers).await?;

        #[cfg(debug_assertions)]
        {
            let elapsed = start.elapsed();
            debug!(
                "Resolved host header '{}' in {:?} using configured strategy",
                resolved.header, elapsed
            );
        }

        Ok(resolved)
    }

    /// Resolves a file path using the filesystem abstraction and the provided route
    pub async fn resolve_file_from_path(
        &self,
        path: &str,
    ) -> Result<String, crate::error::ServerError> {
        debug!("Resolving file from path: {}", path);
        unimplemented!()
    }

    /// The main function that handles incoming requests.
    async fn handle_request(
        &self,
        req: Request<IncomingBody>,
    ) -> Result<Response<Full<Bytes>>, ServerError> {
        use chrono::prelude::*;

        info!(
            "[{}] {} {} - {}",
            Utc::now().to_rfc3339(),
            req.method(),
            req.uri(),
            req.headers()
                .get("User-Agent")
                .unwrap_or(&HeaderValue::from_static("Unknown"))
                .to_str()
                .unwrap_or("Unknown")
        );

        let headers = req.headers();
        trace!("Request headers: {:?}", headers);

        let resolved = self.resolve_host(headers).await?;
        trace!("Resolved host: {:?}", resolved.host);

        // For now, we will only cache the resolved header if we are in auto-detect mode.
        if resolved.is_auto {
            let mut config = self.config.write().await;
            config.set_resolved_host_header(resolved.header.clone());
            debug!("Cached target header: {}", resolved.header);
        } else {
            debug!("Not caching target header, auto-detect mode is disabled");
        }

        // TODO: handle redirect

        // Redirects take precedence over rewrites, we need to check for that first before
        // any attempt to normalize the path (with index.html for example) or rewrite it

        // TODO: handle rewrites

        unimplemented!()
    }
}

impl HyperService<Request<IncomingBody>> for Service {
    type Response = Response<Full<Bytes>>;
    type Error = ServerError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        let service = self.clone();
        Box::pin(async move { service.handle_request(req).await })
    }
}

pub enum Status {
    Ok {
        /// The body of the response
        body: String,

        /// The headers to include in the response
        headers: HeaderMap<HeaderValue>,
    },
    NotFound,
    InternalServerError,
    BadRequest,
    Redirect {
        /// The target URL or path to redirect to
        target: String,
    },
    GenericError {
        /// The error message to include in the response
        message: String,

        /// The HTTP status code to return
        code: StatusCode,

        /// Additional headers to include in the response
        headers: HeaderMap<HeaderValue>,
    },
}

const NOT_FOUND: &str = "Not Found";
const INTERNAL_SERVER_ERROR: &str = "Internal Server Error";
const BAD_REQUEST: &str = "Bad Request";

impl Service {
    fn respond(&self, status: Status) -> Response<Full<Bytes>> {
        match status {
            Status::Ok { body, headers } => {
                let mut response = Response::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::from(body)))
                    .unwrap();

                // Add headers to the response
                for (key, value) in headers.iter() {
                    response.headers_mut().insert(key.clone(), value.clone());
                }

                response
            }
            Status::NotFound => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from(NOT_FOUND)))
                .unwrap(),
            Status::InternalServerError => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(INTERNAL_SERVER_ERROR)))
                .unwrap(),
            Status::BadRequest => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(BAD_REQUEST)))
                .unwrap(),
            Status::Redirect { target } => {
                let mut response = Response::builder()
                    .status(StatusCode::FOUND) // Default to 302 Found
                    .body(Full::new(Bytes::from(format!("Redirecting to {}", target))))
                    .unwrap();

                // Set the Location header for the redirect
                response
                    .headers_mut()
                    .insert(header::LOCATION, HeaderValue::from_str(&target).unwrap());

                response
            }
            Status::GenericError {
                message,
                code,
                headers,
            } => {
                let mut response = Response::builder()
                    .status(code)
                    .body(Full::new(Bytes::from(message)))
                    .unwrap();

                // Add headers to the response
                for (key, value) in headers.iter() {
                    response.headers_mut().insert(key.clone(), value.clone());
                }

                response
            }
        }
    }

    /// Redirects to the specified target URL or path.
    fn handle_redirect(&self, rule: RedirectRule) -> Result<Response<Full<Bytes>>, ServerError> {
        debug!("Redirecting to: {}", rule.target());

        let status = match (rule.is_temporary(), rule.is_replay()) {
            // Temporary + replay
            (true, true) => StatusCode::TEMPORARY_REDIRECT, // 307 Temporary Redirect
            // Permanent + replay
            (false, true) => StatusCode::PERMANENT_REDIRECT, // 308 Permanent Redirect
            // Temporary + not replay
            (true, false) => StatusCode::FOUND, // 302 Found
            // Permanent + not replay
            (false, false) => StatusCode::MOVED_PERMANENTLY, // 301 Moved Permanently
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            header::LOCATION,
            HeaderValue::from_str(&rule.target()).map_err(|e| ServerError::InvalidHeaderValue {
                header: "Location".to_string(),
                value: rule.target().to_string(),
                message: e.to_string(),
            })?,
        );

        debug!("Redirecting to: {}, status: {}", rule.target(), status);
        Ok(self.respond(Status::Redirect {
            target: rule.target().to_string(),
        }))
    }

    fn handle_rewrite(&self, rule: RewriteRule) -> Result<Response<Full<Bytes>>, ServerError> {
        debug!("Rewriting to: {}", rule.target());

        unimplemented!("file resolution logic has not been implemented yet")
    }
}
