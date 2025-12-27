use http_body_util::Full;
use hyper::body::Bytes;
use hyper::header::{self, HeaderName, HeaderValue};
use hyper::service::Service as HyperService;
use hyper::{HeaderMap, StatusCode};
use hyper::{Request, Response, body::Incoming as IncomingBody};
use log::{debug, info, trace};
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

use crate::config::{ConfigHandle, RedirectRule, Site};
use crate::error::ServerError;
use crate::filesystem::FilesystemError;
use crate::server::mimetype;
use crate::with_leading_slash;

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
#[allow(dead_code)]
pub struct Service {
    /// The filesystem abstraction used by the server
    filesystem: Arc<dyn crate::filesystem::Filesystem>,

    /// The configuration for the server
    config: ConfigHandle,
}

impl Service {
    pub fn new(filesystem: Arc<dyn crate::filesystem::Filesystem>, config: ConfigHandle) -> Self {
        debug!("Creating a new Resolver instance");
        Service { filesystem, config }
    }

    /// Resolves the host from the request headers using the cached resolved host header.
    pub async fn resolve_host_with_cache(
        &self,
        headers: &HeaderMap<HeaderValue>,
    ) -> Result<DetectedHost, crate::error::ServerError> {
        let config = self.config.receiver.borrow().clone();

        let resolved_header_name = config
            .resolved_host_header()
            .ok_or(crate::error::ServerError::MissingResolvedHostHeader)?;

        debug!("Using cached resolved host header: {resolved_header_name}",);

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
                    "Cached header '{resolved_header_name}' is not a valid UTF-8 string",
                ),
            });
        }

        Err(crate::error::ServerError::HostDetectionFailed {
            message: format!("Cached header '{resolved_header_name}' not found in request headers",),
        })
    }

    /// Resolves the host from the request headers using the configured host detection strategy.
    pub async fn resolve_host_with_strategy(
        &self,
        headers: &HeaderMap<HeaderValue>,
    ) -> Result<DetectedHost, crate::error::ServerError> {
        let config = self.config.get();
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

                    debug!("Header '{header}' is not a valid UTF-8 string header");
                }
                None => {
                    debug!("Header '{header}' not found in request");
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

        let config = self.config.get();

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
    pub async fn resolve_file_from_route(
        &self,
        route: &str,
        site: &Site,
    ) -> Result<Option<PathBuf>, crate::error::ServerError> {
        let route = route.trim_matches('/').to_string();

        // Use the site's root directory (already set to full path in CLI)
        let path = PathBuf::from(&site.root);
        debug!(
            "Base path for site {}: {}",
            site.name,
            path.to_string_lossy()
        );
        debug!(
            "Resolving file for site: {}, path: {}",
            site.name,
            path.join(&route).to_string_lossy()
        );

        // Check the stat of the path to determine if it exists and what type it is
        let stat = match self.filesystem.stat(path.join(&route)) {
            Ok(stat) => stat,
            Err(FilesystemError::NotFound(_)) => {
                return Ok(None);
            }
            Err(e) => {
                debug!("Failed to stat path: {route}, error: {e}");
                return Err(ServerError::FilesystemError(e));
            }
        };

        // We need to first normalize to an index file if any of the following conditions are met:
        // - the path is empty
        // - the path is a forward slash
        // - the path is a directory
        let path = if stat.is_directory() || route.trim_matches('/').is_empty() {
            debug!("Attaching directory to path: {route}");
            let path = path.join(&route);

            debug!("Path is a directory or empty, resolving to index file");
            // We will resolve to the index file of the site, if it exists.
            let dir_index_file = path.join(site.index_file());
            debug!(
                "Resolving to index file in directory: {}",
                dir_index_file.to_string_lossy()
            );

            match self.filesystem.exists(dir_index_file.clone()) {
                Ok(true) => dir_index_file.to_string_lossy().to_string(),
                _ => return Ok(None),
            }
        } else {
            path.join(route).to_string_lossy().to_string()
        };

        debug!("Resolved file path: {path}");

        // We need to make sure what we are dealing with even exists before we return it.
        if !self
            .filesystem
            .exists(path.clone().into())
            .map_err(ServerError::FilesystemError)?
        {
            debug!("Path does not exist: {path:?}");
            return Ok(None);
        }

        Ok(Some(path.into()))
    }

    /// The main function that handles incoming requests.
    async fn handle_request(
        &self,
        req: Request<IncomingBody>,
    ) -> Result<Response<Full<Bytes>>, ServerError> {
        #[cfg(debug_assertions)]
        let start = std::time::Instant::now();

        let config = self.config.get();

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
        trace!("Request headers: {headers:?}");

        let resolved = self.resolve_host(headers).await?;
        trace!("Resolved host: {:?}", resolved.host);

        // For now, we will only cache the resolved header if we are in auto-detect mode.
        if resolved.is_auto {
            debug!("Acquiring configuration for writing resolved host header");

            let mut new_config = (*config).clone();
            new_config.set_resolved_host_header(resolved.header.clone());

            if let Err(e) = self.config.set(new_config) {
                debug!("Failed to update configuration with resolved host header: {e}");
                return Err(e);
            }

            debug!("Cached target header: {}", resolved.header);
        } else {
            debug!("Not caching target header, auto-detect mode is disabled");
        }

        let site = config
            .sites
            .find_by_hostname(&resolved.host)
            .ok_or_else(|| ServerError::SiteNotFound {
                host: resolved.host.clone(),
            })?;
        let path = with_leading_slash!(req.uri().path());

        // Redirects take precedence over rewrites, we need to check for that first before
        // any attempt to normalize the path (with index.html for example) or rewrite it
        if let Some(rule) = site.find_redirect_rule(path.as_str()) {
            debug!("Found redirect rule for path: {}", req.uri().path());
            return self.handle_redirect(rule);
        }

        // We need to check for possible rewrite rules, since if there are any, we need to use the
        // configured rewrite path going forward.
        let path = site
            .find_rewrite_rule(path.as_str())
            .map_or(path.to_string(), |rule| rule.target().to_string());

        debug!("Resolved path after rewrites: {path}");

        let file = self.resolve_file_from_route(&path, site).await?;

        match file {
            Some(file) => {
                debug!("Resolved file: {file:?}");
                let response = self.respond_with_file(file, site);

                #[cfg(debug_assertions)]
                {
                    let elapsed = start.elapsed();
                    debug!(
                        "Handled request for {} in {:?} with response: {:?}",
                        req.uri().path(),
                        elapsed,
                        response.as_ref().map(|r| r.status()),
                    );
                }

                response
            }
            None => {
                info!("File not found for route: {}", req.uri().path());

                // If there is a fallback file configured, we will try to serve that instead.
                if let Some(fallback) = &site.fallback_file {
                    debug!("Serving fallback file: {fallback}");
                    let fallback_path = PathBuf::from(&config.sites_directory)
                        .join(&site.name)
                        .join(fallback);

                    debug!(
                        "Checking for fallback file at: {}",
                        fallback_path.to_string_lossy()
                    );

                    if let Ok(true) = self.filesystem.exists(fallback_path.clone()) {
                        return self.respond_with_file(fallback_path, site);
                    }
                }

                Ok(self.respond(Status::NotFound))
            }
        }
    }

    /// Handles errors that occur during request processing.
    fn handle_error(&self, error: ServerError) -> Response<Full<Bytes>> {
        debug!("Handling error: {error}");
        let status = match error {
            ServerError::SiteNotFound { host } => {
                if cfg!(debug_assertions) {
                    Status::GenericError {
                        message: format!("No site found for host: {host}"),
                        code: StatusCode::NOT_FOUND,
                        headers: HeaderMap::new(),
                    }
                } else {
                    Status::NotFound
                }
            }
            ServerError::InvalidHeaderValue {
                header,
                value,
                message,
            } => Status::GenericError {
                message: format!(
                    "Invalid header value for '{header}': '{value}', reason: {message}"
                ),
                code: StatusCode::BAD_REQUEST,
                headers: HeaderMap::new(),
            },
            _ => Status::InternalServerError,
        };

        self.respond(status)
    }
}

pub enum Status {
    Ok {
        /// The body of the response
        body: Vec<u8>,

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
                    .body(Full::new(Bytes::from(format!("Redirecting to {target}"))))
                    .unwrap();

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

                for (key, value) in headers.iter() {
                    response.headers_mut().insert(key.clone(), value.clone());
                }

                response
            }
        }
    }

    /// Responds with a file from the filesystem, setting the appropriate headers.
    pub fn respond_with_file(
        &self,
        file: PathBuf,
        site: &Site,
    ) -> Result<Response<Full<Bytes>>, ServerError> {
        let mime_type = mimetype::from_path(file.clone());
        let content = self
            .filesystem
            .read_file(file)
            .map_err(ServerError::FilesystemError)?;

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(mime_type).unwrap(),
        );

        site.response_headers.iter().for_each(|(key, value)| {
            if let Ok(header_name) = HeaderName::from_str(key) {
                headers.insert(header_name, HeaderValue::from_str(value).unwrap());
            }
        });

        Ok(self.respond(Status::Ok {
            body: content.bytes().to_vec(),
            headers,
        }))
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
}

impl HyperService<Request<IncomingBody>> for Service {
    type Response = Response<Full<Bytes>>;
    type Error = ServerError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        let service = self.clone();
        Box::pin(async move {
            match service.handle_request(req).await {
                Ok(response) => Ok(response),
                Err(e) => Ok(service.handle_error(e)),
            }
        })
    }
}
