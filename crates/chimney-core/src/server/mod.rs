pub mod mimetype;
pub mod redirect;
pub mod service;

use std::{net::SocketAddr, sync::Arc};

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use log::{debug, error, info};

use crate::{
    config::{Config, ConfigHandle},
    error::ServerError,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Notify,
};

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
    tls_manager: Option<Arc<crate::tls::TlsManager>>,

    /// TLS acceptor with SNI support (if TLS is enabled)
    tls_acceptor: Option<Arc<tokio_rustls::TlsAcceptor>>,
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
    pub fn new(filesystem: Arc<dyn crate::filesystem::Filesystem>, config: Arc<Config>) -> Self {
        debug!("Creating a new Chimney server instance");

        let (config_tx, config_rx) = tokio::sync::watch::channel(config.clone());
        let config_handle = ConfigHandle::new(config_tx, config_rx);

        let service = service::Service::new(filesystem.clone(), config_handle.clone());

        Server {
            config_handle,
            signal: Arc::new(Notify::new()),
            graceful_shutdown: true,
            service,
            tls_manager: None,
            tls_acceptor: None,
        }
    }

    /// Create a new server instance with TLS support enabled
    ///
    /// This constructor automatically detects HTTPS configuration and initializes:
    /// - ACME certificate management (if `auto_issue = true` in any site)
    /// - Manual certificate loading (if certificate files are provided)
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
        config: Arc<Config>,
    ) -> Result<Self, ServerError> {
        debug!("Creating a new Chimney server instance with TLS");

        let (config_tx, config_rx) = tokio::sync::watch::channel(config.clone());
        let config_handle = ConfigHandle::new(config_tx, config_rx);

        let service = service::Service::new(filesystem.clone(), config_handle.clone());

        // Initialize TLS if any site has HTTPS enabled
        let (tls_manager, tls_acceptor) = if crate::tls::TlsManager::is_tls_enabled(&config) {
            info!("TLS is enabled, initializing TLS manager");
            let manager = Arc::new(crate::tls::TlsManager::new(config.clone()).await?);

            // Only build manual TLS acceptor if we have manual certificates and no ACME
            let acceptor = if !manager.has_acme() && !manager.is_manual_empty() {
                Some(manager.build_acceptor()?)
            } else {
                None
            };

            (Some(manager), acceptor)
        } else {
            (None, None)
        };

        Ok(Server {
            config_handle,
            signal: Arc::new(Notify::new()),
            graceful_shutdown: true,
            service,
            tls_manager,
            tls_acceptor,
        })
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
        if self.tls_acceptor.is_some() {
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
        info!("HTTP server listening on {}", socket_addr);

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
        let http_addr = format!("{}:{}", config.host, http_port)
            .parse::<SocketAddr>()
            .map_err(|e| ServerError::InvalidRawSocketAddress {
                address: format!("{}:{}", config.host, http_port),
                message: e.to_string(),
            })?;
        let http_listener = TcpListener::bind(http_addr)
            .await
            .map_err(ServerError::FailedToBind)?;
        info!("HTTP server listening on {}", http_addr);

        // Create HTTPS listener (port 443)
        let https_port = 443;
        let https_addr = format!("{}:{}", config.host, https_port)
            .parse::<SocketAddr>()
            .map_err(|e| ServerError::InvalidRawSocketAddress {
                address: format!("{}:{}", config.host, https_port),
                message: e.to_string(),
            })?;
        let https_listener = TcpListener::bind(https_addr)
            .await
            .map_err(ServerError::FailedToBind)?;
        info!("HTTPS server listening on {}", https_addr);

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
        let redirect_svc =
            redirect::RedirectService::new(self.service.clone(), self.config_handle.clone(), is_https);

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

        // Check if we're using ACME
        let tls_manager = self
            .tls_manager
            .as_ref()
            .ok_or(ServerError::TlsNotConfigured)?;

        if tls_manager.has_acme() {
            // ACME mode - use AcmeAcceptor
            self.handle_acme_connection(stream, addr).await?;
        } else {
            // Manual certificate mode - use regular TLS acceptor
            self.handle_manual_tls_connection(stream, addr).await?;
        }

        Ok(())
    }

    /// Handle HTTPS connection with ACME acceptor
    async fn handle_acme_connection(
        &self,
        stream: TcpStream,
        addr: SocketAddr,
    ) -> Result<(), ServerError> {
        let tls_manager = self
            .tls_manager
            .as_ref()
            .ok_or(ServerError::TlsNotConfigured)?;

        let acme_acceptor = tls_manager
            .acme_acceptor()
            .ok_or(ServerError::TlsNotConfigured)?;

        // Get the ACME resolver for certificate resolution
        let acme_resolver = tls_manager
            .acme_resolver()
            .ok_or(ServerError::TlsNotConfigured)?;

        // Use redirect service with is_https=true (won't redirect)
        let redirect_svc =
            redirect::RedirectService::new(self.service.clone(), self.config_handle.clone(), true);

        // Clone the acceptor for the async task
        let acme_acceptor = acme_acceptor.clone();

        // Perform ACME accept and serve in a separate task
        tokio::spawn(async move {
            // Accept with ACME acceptor
            match acme_acceptor.accept(stream).await {
                Ok(None) => {
                    // ACME TLS-ALPN-01 validation request was handled
                    debug!("Handled ACME TLS-ALPN-01 validation request from {addr}");
                }
                Ok(Some(start_handshake)) => {
                    // Regular TLS connection - complete the handshake
                    debug!("Starting TLS handshake for regular connection from {addr}");

                    // Complete the TLS handshake with the ACME resolver
                    let server_config = rustls::ServerConfig::builder()
                        .with_no_client_auth()
                        .with_cert_resolver(acme_resolver);

                    match start_handshake
                        .into_stream(std::sync::Arc::new(server_config))
                        .await
                    {
                        Ok(tls_stream) => {
                            debug!("TLS handshake successful for {addr}");

                            let io = TokioIo::new(tls_stream);

                            // Serve the connection over TLS
                            if let Err(err) =
                                http1::Builder::new().serve_connection(io, redirect_svc).await
                            {
                                error!("Failed to serve HTTPS connection: {err:?}");
                            }
                        }
                        Err(e) => {
                            error!("TLS handshake failed for {addr}: {e}");
                        }
                    }
                }
                Err(e) => {
                    error!("ACME accept failed for {addr}: {e}");
                }
            }
        });

        Ok(())
    }

    /// Handle HTTPS connection with manual TLS certificate
    async fn handle_manual_tls_connection(
        &self,
        stream: TcpStream,
        addr: SocketAddr,
    ) -> Result<(), ServerError> {
        let tls_acceptor = self
            .tls_acceptor
            .as_ref()
            .ok_or(ServerError::TlsNotConfigured)?
            .clone();

        // Use redirect service with is_https=true (won't redirect)
        let redirect_svc =
            redirect::RedirectService::new(self.service.clone(), self.config_handle.clone(), true);

        // Perform TLS handshake and serve in a separate task
        tokio::spawn(async move {
            // Perform TLS handshake
            let tls_stream = match tls_acceptor.accept(stream).await {
                Ok(stream) => stream,
                Err(e) => {
                    error!("TLS handshake failed for {addr}: {e}");
                    return;
                }
            };

            debug!("TLS handshake successful for {addr}");

            let io = TokioIo::new(tls_stream);

            // Serve the connection over TLS
            if let Err(err) = http1::Builder::new().serve_connection(io, redirect_svc).await {
                error!("Failed to serve HTTPS connection: {err:?}");
            }
        });

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
