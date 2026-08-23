mod hooks;
pub mod mimetype;
pub mod redirect;
pub mod service;

pub use hooks::{RequestEvent, RequestProtocol};

use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use log::{debug, error, info};

use crate::{config::ConfigHandle, error::ServerError};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, Notify},
};
use tokio_rustls::TlsAcceptor;

const SHUTDOWN_WAIT_PERIOD: u64 = 15; // seconds

pub struct Server {
    /// The configuration for the server
    config_handle: ConfigHandle,

    /// The shutdown signal for the server
    signal: Arc<Notify>,

    /// Whether to shut down gracefully (default: true)
    graceful_shutdown: bool,

    /// The service for handling requests
    service: service::Service,

    /// TLS manager for handling certificates and ACME (if TLS is enabled)
    tls: RwLock<Option<TlsRuntime>>,

    /// Serializes transactional configuration reloads.
    reload_lock: Mutex<()>,

    /// Callbacks invoked after a request response has been produced.
    request_hooks: hooks::RequestHooks,
}

#[derive(Clone)]
struct TlsRuntime {
    manager: Arc<crate::tls::TlsManager>,
    manual_acceptor: Option<Arc<tokio_rustls::TlsAcceptor>>,
}

impl Server {
    /// Create a new Chimney server instance without TLS support
    ///
    /// Use this constructor for HTTP-only servers. If you need HTTPS support
    /// (either manual certificates or ACME), use [`Server::new_with_tls`] instead.
    ///
    /// # Example (Library Usage)
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use chimney::{Config, Server, filesystem::LocalFilesystem};
    ///
    /// let filesystem = Arc::new(LocalFilesystem::new());
    /// let config = Arc::new(Config::default());
    /// let server = Server::new(filesystem, config);
    /// ```
    pub fn new(
        filesystem: Arc<dyn crate::filesystem::Filesystem>,
        config_handle: ConfigHandle,
    ) -> Self {
        debug!("Creating a new Chimney server instance");
        let service = service::Service::new(filesystem.clone(), config_handle.clone());

        Server {
            config_handle,
            signal: Arc::new(Notify::new()),
            graceful_shutdown: true,
            service,
            tls: RwLock::new(None),
            reload_lock: Mutex::new(()),
            request_hooks: hooks::RequestHooks::default(),
        }
    }

    /// Create a new server instance with TLS support enabled
    ///
    /// This constructor automatically detects HTTPS configuration and initializes:
    /// - ACME certificate management (for sites without manual certificates)
    /// - Manual certificate loading (for sites with `cert_file` and `key_file`)
    /// - SNI for multi-domain support
    ///
    /// Use this instead of [`Server::new`] when any site has HTTPS enabled.
    ///
    /// # Example (Library Usage)
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use chimney::{Config, Server, filesystem::LocalFilesystem};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let filesystem = Arc::new(LocalFilesystem::new());
    ///     let config = Arc::new(Config::default());
    ///
    ///     // Automatically handles ACME and manual certificates based on site config
    ///     let server = Server::new_with_tls(filesystem, config).await?;
    ///
    ///     server.run().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new_with_tls(
        filesystem: Arc<dyn crate::filesystem::Filesystem>,
        config_handle: ConfigHandle,
    ) -> Result<Self, ServerError> {
        debug!("Creating a new Chimney server instance with TLS");
        let config = config_handle.get();
        let service = service::Service::new(filesystem.clone(), config_handle.clone());

        // Initialize TLS if any site has HTTPS enabled
        let tls = if crate::tls::TlsManager::is_tls_enabled(&config) {
            info!("TLS is enabled, initializing TLS manager");
            let manager = Arc::new(crate::tls::TlsManager::new(config).await?);

            // Only build manual TLS acceptor if we have manual certificates and no ACME
            let acceptor = if !manager.has_acme() && !manager.is_manual_empty() {
                Some(manager.build_acceptor()?)
            } else {
                None
            };

            Some(TlsRuntime {
                manager,
                manual_acceptor: acceptor,
            })
        } else {
            None
        };

        Ok(Server {
            config_handle,
            signal: Arc::new(Notify::new()),
            graceful_shutdown: true,
            service,
            tls: RwLock::new(tls),
            reload_lock: Mutex::new(()),
            request_hooks: hooks::RequestHooks::default(),
        })
    }

    /// Register a callback invoked after Chimney has produced each response.
    ///
    /// Hooks are shared by HTTP and HTTPS listeners, survive configuration
    /// reloads, and are isolated from the request path if they panic. Keep hook
    /// implementations short; for blocking I/O, send the event to a worker.
    pub fn on_request<F>(&self, hook: F)
    where
        F: Fn(&RequestEvent) + Send + Sync + 'static,
    {
        self.request_hooks.add(hook);
    }

    /// Atomically reload routing, global settings, and TLS certificate state.
    ///
    /// TLS and manual certificate files are fully validated before the live
    /// configuration is changed. Existing connections retain their current TLS
    /// snapshot. Listener address, port, or HTTP/TLS topology changes require a
    /// process restart because reload never binds a second listener.
    pub async fn reload(&self, config: crate::config::Config) -> Result<(), ServerError> {
        let _guard = self.reload_lock.lock().await;
        let current = self.config_handle.get();
        Self::validate_listener_compatibility(&current, &config)?;

        let next_tls = if crate::tls::TlsManager::is_tls_enabled(&config) {
            let manager = Arc::new(crate::tls::TlsManager::new(Arc::new(config.clone())).await?);
            let manual_acceptor = if !manager.has_acme() && !manager.is_manual_empty() {
                Some(manager.build_acceptor()?)
            } else {
                None
            };
            Some(TlsRuntime {
                manager,
                manual_acceptor,
            })
        } else {
            None
        };

        // Publish TLS first so a newly routed hostname can never observe the old
        // certificate resolver. Roll it back if the watch channel rejects the
        // configuration update.
        let previous_tls = {
            let mut tls = self.tls.write().expect("TLS state lock poisoned");
            std::mem::replace(&mut *tls, next_tls)
        };
        if let Err(error) = self.config_handle.set(config) {
            *self.tls.write().expect("TLS state lock poisoned") = previous_tls;
            return Err(error);
        }
        Ok(())
    }

    fn validate_listener_compatibility(
        current: &crate::config::Config,
        next: &crate::config::Config,
    ) -> Result<(), ServerError> {
        let current_tls = crate::tls::TlsManager::is_tls_enabled(current);
        let next_tls = crate::tls::TlsManager::is_tls_enabled(next);
        let current_https_port = current.https.as_ref().map(|https| https.port);
        let next_https_port = next.https.as_ref().map(|https| https.port);
        if current.host != next.host || current.port != next.port {
            return Err(ServerError::ReloadRequiresRestart(
                "HTTP host or port changed".to_string(),
            ));
        }
        if current_tls != next_tls {
            return Err(ServerError::ReloadRequiresRestart(
                "HTTP/TLS listener topology changed".to_string(),
            ));
        }
        if current_tls && current_https_port != next_https_port {
            return Err(ServerError::ReloadRequiresRestart(
                "HTTPS port changed".to_string(),
            ));
        }
        Ok(())
    }

    pub fn set_graceful_shutdown(&mut self, graceful: bool) {
        debug!("Setting graceful shutdown to {graceful}");
        self.graceful_shutdown = graceful;
    }

    /// Watch for a shutdown signal (like Ctrl+C) and notify the server to shut down gracefully.
    async fn watch_for_shutdown(&self) {
        if !self.graceful_shutdown {
            debug!("Graceful shutdown is disabled, skipping signal watcher");
            return;
        }

        debug!("Setting up Ctrl+C signal handler for graceful shutdown");

        let signal = Arc::clone(&self.signal);
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");

            info!("Received shutdown signal, shutting down the server...");
            signal.notify_waiters();
        });
    }

    /// Get the socket address for the server based on the configuration.
    pub async fn get_socket_address(&self) -> Result<SocketAddr, ServerError> {
        let config = self.config_handle.get();

        let raw_addr = format!("{}:{}", config.host, config.port);
        debug!("Parsing socket address: {raw_addr}");
        raw_addr
            .parse::<SocketAddr>()
            .map_err(|e| ServerError::InvalidRawSocketAddress {
                address: raw_addr.clone(),
                message: e.to_string(),
            })
    }

    /// Run the main server loop to accept and handle incoming connections.
    pub async fn run(&self) -> Result<(), ServerError> {
        debug!("Starting Chimney server...");

        self.watch_for_shutdown().await;

        // Determine if we need dual listeners (HTTP + HTTPS)
        // Check for tls_manager to support both ACME and manual certificates
        if self.tls.read().expect("TLS state lock poisoned").is_some() {
            info!("TLS is enabled, starting dual listeners (HTTP + HTTPS)");
            self.run_dual_listeners().await
        } else {
            info!("Running HTTP-only server");
            self.run_http_only().await
        }
    }

    /// Run HTTP-only server (no TLS)
    async fn run_http_only(&self) -> Result<(), ServerError> {
        let listener = self.make_tcp_listener().await?;
        let socket_addr = self.get_socket_address().await?;
        info!("HTTP server listening on {socket_addr}");

        // Graceful shutdown handling for the Hyper server
        let graceful = hyper_util::server::graceful::GracefulShutdown::new();

        loop {
            tokio::select! {
                _ = self.signal.notified() => {
                    drop(listener);
                    debug!("Shutdown signal received, exiting server loop");
                    break;
                }

                connection = listener.accept() => {
                    self.handle_http_connection(connection, &graceful).await?;
                }
            }
        }

        // Start graceful shutdown watcher when the main loop is broken
        tokio::select! {
            _ = graceful.shutdown() => {
                debug!("Closed all connections gracefully");
                Ok(())
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(SHUTDOWN_WAIT_PERIOD)) => {
                error!("Timed out wait for all connections to close");
                Err(ServerError::TimeoutWaitingForConnections)
            }
        }
    }

    /// Run dual listeners for HTTP and HTTPS
    async fn run_dual_listeners(&self) -> Result<(), ServerError> {
        let config = self.config_handle.get();

        // Create HTTP listener (configured port)
        let http_port = config.port;
        let http_addr = format!("{}:{http_port}", config.host)
            .parse::<SocketAddr>()
            .map_err(|e| {
                let host = &config.host;
                ServerError::InvalidRawSocketAddress {
                    address: format!("{host}:{http_port}"),
                    message: e.to_string(),
                }
            })?;
        let http_listener = TcpListener::bind(http_addr)
            .await
            .map_err(ServerError::FailedToBind)?;
        info!("HTTP server listening on {http_addr}");

        // Create HTTPS listener (port 443)
        let https_port = if let Some(https_cfg) = &config.https {
            https_cfg.port
        } else {
            443
        };
        let https_addr = format!("{}:{https_port}", config.host)
            .parse::<SocketAddr>()
            .map_err(|e| {
                let host = &config.host;
                ServerError::InvalidRawSocketAddress {
                    address: format!("{host}:{https_port}"),
                    message: e.to_string(),
                }
            })?;
        let https_listener = TcpListener::bind(https_addr)
            .await
            .map_err(ServerError::FailedToBind)?;
        info!("HTTPS server listening on {https_addr}");

        // Graceful shutdown handling
        let graceful = hyper_util::server::graceful::GracefulShutdown::new();

        loop {
            tokio::select! {
                _ = self.signal.notified() => {
                    drop(http_listener);
                    drop(https_listener);
                    debug!("Shutdown signal received, exiting server loop");
                    break;
                }

                connection = http_listener.accept() => {
                    self.handle_http_connection(connection, &graceful).await?;
                }

                connection = https_listener.accept() => {
                    self.handle_https_connection(connection, &graceful).await?;
                }
            }
        }

        // Start graceful shutdown watcher
        tokio::select! {
            _ = graceful.shutdown() => {
                debug!("Closed all connections gracefully");
                Ok(())
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(SHUTDOWN_WAIT_PERIOD)) => {
                error!("Timed out wait for all connections to close");
                Err(ServerError::TimeoutWaitingForConnections)
            }
        }
    }

    /// Handle HTTP connection with optional redirect to HTTPS
    async fn handle_http_connection(
        &self,
        connection: Result<(TcpStream, SocketAddr), std::io::Error>,
        graceful: &hyper_util::server::graceful::GracefulShutdown,
    ) -> Result<(), ServerError> {
        let (stream, addr) = connection.map_err(ServerError::FailedToAcceptConnection)?;
        debug!("Accepted HTTP connection from {addr}");

        let io = TokioIo::new(stream);

        // Always use redirect service - it will only redirect if TLS is enabled and auto_redirect is true
        let is_https = false;
        let redirect_svc = redirect::RedirectService::new_with_context(
            self.service.clone(),
            self.config_handle.clone(),
            is_https,
            Some(addr),
            self.request_hooks.clone(),
        );

        let conn = http1::Builder::new().serve_connection(io, redirect_svc);
        let fut = graceful.watch(conn);

        tokio::spawn(async move {
            if let Err(err) = fut.await {
                error!("Failed to serve HTTP connection: {err:?}");
            }
        });

        Ok(())
    }

    /// Handle HTTPS connection with TLS handshake
    async fn handle_https_connection(
        &self,
        connection: Result<(TcpStream, SocketAddr), std::io::Error>,
        _graceful: &hyper_util::server::graceful::GracefulShutdown,
    ) -> Result<(), ServerError> {
        let (stream, addr) = connection.map_err(ServerError::FailedToAcceptConnection)?;
        debug!("Accepted HTTPS connection from {addr}");

        let tls = self
            .tls
            .read()
            .expect("TLS state lock poisoned")
            .clone()
            .ok_or(ServerError::TlsNotConfigured)?;
        let tls_manager = tls.manager;

        // Use redirect service with is_https=true (won't redirect)
        let redirect_svc = redirect::RedirectService::new_with_context(
            self.service.clone(),
            self.config_handle.clone(),
            true,
            Some(addr),
            self.request_hooks.clone(),
        );

        if tls_manager.has_acme() {
            // ACME mode - use AcmeAcceptor for TLS-ALPN-01 challenge handling
            let acme_acceptor = tls_manager
                .acme_acceptor()
                .ok_or(ServerError::TlsNotConfigured)?
                .clone();

            let resolver_for_handshake = tls_manager
                .acme_resolver()
                .ok_or(ServerError::TlsNotConfigured)?;

            // Perform ACME accept and serve in a separate task
            tokio::spawn(async move {
                if let Err(e) = Self::handle_acme_connection_inner(
                    stream,
                    addr,
                    acme_acceptor,
                    resolver_for_handshake,
                    redirect_svc,
                )
                .await
                {
                    error!("ACME connection handler failed for {addr}: {e}");
                }
            });
        } else {
            // Manual certificate mode - use regular TLS acceptor
            let tls_acceptor = tls
                .manual_acceptor
                .as_ref()
                .ok_or(ServerError::TlsNotConfigured)?
                .clone();

            // Perform TLS handshake and serve in a separate task
            tokio::spawn(async move {
                if let Err(e) = Self::handle_manual_tls_connection_inner(
                    stream,
                    addr,
                    tls_acceptor,
                    redirect_svc,
                )
                .await
                {
                    error!("Manual TLS connection handler failed for {addr}: {e}");
                }
            });
        }

        Ok(())
    }

    /// Handle ACME connection inner logic (extracted for better error handling)
    async fn handle_acme_connection_inner(
        stream: TcpStream,
        addr: SocketAddr,
        acme_acceptor: tokio_rustls_acme::AcmeAcceptor,
        resolver_for_handshake: Arc<dyn rustls::server::ResolvesServerCert>,
        redirect_svc: redirect::RedirectService,
    ) -> Result<(), ServerError> {
        match acme_acceptor.accept(stream).await {
            Ok(None) => {
                // ACME TLS-ALPN-01 validation request was handled
                debug!("Handled ACME TLS-ALPN-01 validation request from {addr}");
            }
            Ok(Some(start_handshake)) => {
                // Regular TLS connection - complete the handshake
                debug!("Starting TLS handshake for regular connection from {addr}");

                let server_config = rustls::ServerConfig::builder()
                    .with_no_client_auth()
                    .with_cert_resolver(resolver_for_handshake);

                let tls_stream = start_handshake
                    .into_stream(std::sync::Arc::new(server_config))
                    .await
                    .map_err(|e| ServerError::TlsHandshakeFailed(e.to_string()))?;

                debug!("TLS handshake successful for {addr}");

                let io = TokioIo::new(tls_stream);

                // Serve the connection over TLS
                http1::Builder::new()
                    .serve_connection(io, redirect_svc)
                    .await
                    .map_err(|e| {
                        ServerError::TlsHandshakeFailed(format!(
                            "Failed to serve HTTPS connection: {e:?}"
                        ))
                    })?;
            }
            Err(e) => {
                return Err(ServerError::TlsHandshakeFailed(format!(
                    "ACME accept failed: {e}"
                )));
            }
        }

        Ok(())
    }

    /// Handle manual TLS connection inner logic (extracted for better error handling)
    async fn handle_manual_tls_connection_inner(
        stream: TcpStream,
        addr: SocketAddr,
        tls_acceptor: Arc<TlsAcceptor>,
        redirect_svc: redirect::RedirectService,
    ) -> Result<(), ServerError> {
        let tls_stream = tls_acceptor
            .accept(stream)
            .await
            .map_err(|e| ServerError::TlsHandshakeFailed(e.to_string()))?;

        debug!("TLS handshake successful for {addr}");

        let io = TokioIo::new(tls_stream);

        // Serve the connection over TLS
        http1::Builder::new()
            .serve_connection(io, redirect_svc)
            .await
            .map_err(|e| {
                ServerError::TlsHandshakeFailed(format!("Failed to serve HTTPS connection: {e:?}"))
            })?;

        Ok(())
    }

    /// Create the default TCP listener
    async fn make_tcp_listener(&self) -> Result<TcpListener, ServerError> {
        let socket_addr = self.get_socket_address().await?;
        TcpListener::bind(socket_addr)
            .await
            .map_err(ServerError::FailedToBind)
    }
}

#[cfg(test)]
mod reload_tests {
    use super::*;
    use crate::{
        config::{Config, Https, SiteBuilder},
        filesystem::{local::LocalFS, mock::MockFilesystem},
    };
    use std::io::{Read, Write};

    fn http_config() -> Config {
        let mut config = Config::default();
        config.https = None;
        config
    }

    fn server(config: Config) -> Server {
        Server::new(Arc::new(MockFilesystem), config.into())
    }

    #[tokio::test]
    async fn reload_adds_removes_and_updates_site_routing_in_http_mode() {
        let initial = http_config();
        let server = server(initial);

        let mut added = http_config();
        added
            .sites
            .add(
                SiteBuilder::new("site")
                    .domain("new.example")
                    .root("v1")
                    .build(),
            )
            .unwrap();
        server.reload(added).await.unwrap();
        assert_eq!(
            server.config_handle.get().sites.get("site").unwrap().root,
            "v1"
        );

        let mut updated = server.config_handle.get().as_ref().clone();
        updated
            .sites
            .update(
                SiteBuilder::new("site")
                    .domain("new.example")
                    .root("v2")
                    .build(),
            )
            .unwrap();
        server.reload(updated).await.unwrap();
        assert_eq!(
            server.config_handle.get().sites.get("site").unwrap().root,
            "v2"
        );

        let mut removed = server.config_handle.get().as_ref().clone();
        removed.sites.remove("site").unwrap();
        server.reload(removed).await.unwrap();
        assert!(server.config_handle.get().sites.get("site").is_none());
        assert!(server.tls.read().unwrap().is_none());
    }

    #[tokio::test]
    async fn failed_reload_keeps_previous_configuration() {
        let initial = http_config();
        let original_port = initial.port;
        let server = server(initial);
        let mut incompatible = http_config();
        incompatible.port = original_port + 1;

        assert!(matches!(
            server.reload(incompatible).await,
            Err(ServerError::ReloadRequiresRestart(_))
        ));
        assert_eq!(server.config_handle.get().port, original_port);
    }

    #[tokio::test]
    async fn reload_refreshes_acme_domains() {
        let temp = tempfile::tempdir().unwrap();
        let mut initial = Config::default();
        let https = initial.https.as_mut().unwrap();
        https.acme_email = Some("admin@example.com".to_string());
        https.cache_directory = temp.path().to_path_buf();
        initial
            .sites
            .add(SiteBuilder::new("site").domain("old.example").build())
            .unwrap();
        let server = Server::new_with_tls(Arc::new(MockFilesystem), initial.into())
            .await
            .unwrap();

        let mut next = server.config_handle.get().as_ref().clone();
        next.sites
            .update(SiteBuilder::new("site").domain("new.example").build())
            .unwrap();
        server.reload(next).await.unwrap();

        let tls = server.tls.read().unwrap();
        assert_eq!(
            tls.as_ref().unwrap().manager.acme_domains(),
            &["new.example"]
        );
    }

    #[tokio::test]
    async fn reload_refreshes_manual_certificates_and_rejects_invalid_files() {
        let cert = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-https/localhost-cert.pem"
        );
        let key = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-https/localhost-key.pem"
        );
        let manual = || Https {
            auto_redirect: true,
            cert_file: Some(cert.to_string()),
            key_file: Some(key.to_string()),
            ca_file: None,
        };
        let mut initial = Config::default();
        initial
            .sites
            .add(
                SiteBuilder::new("site")
                    .domain("old.example")
                    .https(manual())
                    .build(),
            )
            .unwrap();
        let server = Server::new_with_tls(Arc::new(MockFilesystem), initial.into())
            .await
            .unwrap();

        let mut refreshed = server.config_handle.get().as_ref().clone();
        refreshed
            .sites
            .update(
                SiteBuilder::new("site")
                    .domain("new.example")
                    .https(manual())
                    .build(),
            )
            .unwrap();
        server.reload(refreshed).await.unwrap();
        assert!(
            server
                .tls
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .manual_acceptor
                .is_some()
        );

        let working = server.config_handle.get();
        let mut invalid = working.as_ref().clone();
        invalid
            .sites
            .update(
                SiteBuilder::new("site")
                    .domain("broken.example")
                    .https(Https {
                        auto_redirect: true,
                        cert_file: Some("missing-cert.pem".to_string()),
                        key_file: Some("missing-key.pem".to_string()),
                        ca_file: None,
                    })
                    .build(),
            )
            .unwrap();
        assert!(server.reload(invalid).await.is_err());
        assert_eq!(
            server
                .config_handle
                .get()
                .sites
                .get("site")
                .unwrap()
                .domain_names,
            vec!["new.example"]
        );
    }

    fn send_http_request(port: u16, path: &str) -> String {
        let mut stream = (0..50)
            .find_map(|_| {
                std::net::TcpStream::connect(("127.0.0.1", port))
                    .ok()
                    .or_else(|| {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        None
                    })
            })
            .expect("server did not start");
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: hook.example\r\nUser-Agent: hook-test\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn request_hooks_observe_completed_real_http_requests() {
        let temp = tempfile::tempdir().unwrap();
        let public = temp.path().join("site/public");
        std::fs::create_dir_all(&public).unwrap();
        std::fs::write(public.join("index.html"), "hook response").unwrap();

        let available = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = available.local_addr().unwrap().port();
        drop(available);

        let mut config = http_config();
        config.host = "127.0.0.1".parse().unwrap();
        config.port = port;
        config.sites_directory = temp.path().display().to_string();
        config
            .sites
            .add(
                SiteBuilder::new("site")
                    .domain("hook.example")
                    .root("public")
                    .build(),
            )
            .unwrap();

        let filesystem = Arc::new(LocalFS::new(temp.path().to_path_buf()).unwrap());
        let mut server = Server::new(filesystem, config.into());
        server.set_graceful_shutdown(false);
        let (events_tx, events_rx) = std::sync::mpsc::channel();
        server.on_request(move |event| events_tx.send(event.clone()).unwrap());
        let server = Arc::new(server);
        let running = {
            let server = server.clone();
            tokio::spawn(async move { server.run().await })
        };

        let success = tokio::task::spawn_blocking(move || send_http_request(port, "/"));
        let missing = tokio::task::spawn_blocking(move || send_http_request(port, "/missing"));
        let (success, missing) = tokio::join!(success, missing);
        let success = success.unwrap();
        let missing = missing.unwrap();
        assert!(success.starts_with("HTTP/1.1 200"));
        assert!(missing.starts_with("HTTP/1.1 404"));

        let first = events_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        let second = events_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        let mut statuses = [first.status, second.status];
        statuses.sort_by_key(|status| status.as_u16());
        assert_eq!(
            statuses,
            [hyper::StatusCode::OK, hyper::StatusCode::NOT_FOUND]
        );
        for event in [first, second] {
            assert_eq!(event.method, hyper::Method::GET);
            assert_eq!(event.host.as_deref(), Some("hook.example"));
            assert_eq!(event.site.as_deref(), Some("site"));
            assert_eq!(event.protocol, RequestProtocol::Http);
            assert!(event.remote_addr.is_some());
            assert_eq!(event.request_headers["user-agent"], "hook-test");
            assert!(event.duration > std::time::Duration::ZERO);
        }

        running.abort();
    }
}
