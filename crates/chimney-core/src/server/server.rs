use std::{net::SocketAddr, sync::Arc};

use hyper_util::rt::TokioIo;
use log::{debug, info};

use crate::error::ServerError;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Notify,
};

// TODO: build a domain and sites map to easily lookup sites by domain
pub struct Server {
    /// The filesystem abstraction used by the server
    filesystem: Arc<dyn crate::filesystem::Filesystem>,

    /// The configuration for the server
    config: Arc<crate::config::Config>,

    /// The shutdown signal for the server
    signal: Arc<Notify>,

    /// Whether to shut down gracefully (default: true)
    graceful_shutdown: bool,

    /// The resolver for handling path and resource resolution
    resolver: super::resolver::Resolver,
}

impl Server {
    pub fn new(
        filesystem: Arc<dyn crate::filesystem::Filesystem>,
        config: Arc<crate::config::Config>,
    ) -> Self {
        debug!("Creating a new Chimney server instance");

        Server {
            resolver: super::resolver::Resolver::new(filesystem.clone(), config.clone()),
            filesystem,
            config,
            signal: Arc::new(Notify::new()),
            graceful_shutdown: true,
        }
    }

    /// Get the current configuration of the server.
    pub fn config(&self) -> Arc<crate::config::Config> {
        Arc::clone(&self.config)
    }

    pub fn set_graceful_shutdown(&mut self, graceful: bool) {
        self.graceful_shutdown = graceful;
    }

    /// Watch for a shutdown signal (like Ctrl+C) and notify the server to shut down gracefully.
    async fn watch_for_shutdown(&self) {
        if !self.graceful_shutdown {
            debug!("Graceful shutdown is disabled, skipping signal watcher");
            return;
        }

        let signal = Arc::clone(&self.signal);
        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");

            debug!("Received shutdown signal, shutting down the server...");
            signal.notify_waiters();
        });
    }

    /// Get the socket address for the server based on the configuration.
    pub fn get_socket_address(&self) -> Result<SocketAddr, ServerError> {
        // Prevent the use of possibly reserved ports
        if self.config.port <= 1024 {
            return Err(ServerError::InvalidPortRange {
                port: self.config.port,
            });
        }

        let raw_addr = format!("{}:{}", self.config.host, self.config.port);
        raw_addr
            .parse::<SocketAddr>()
            .map_err(|e| ServerError::InvalidRawSocketAddress {
                address: raw_addr.clone(),
                message: e.to_string(),
            })
    }

    pub async fn run(&self) -> Result<(), ServerError> {
        // Here you would implement the logic to start the server
        // For now, we just print the configuration and return Ok
        debug!("Running with configuration: {:?}", self.config);

        let server = self.make_tcp_listener().await?;

        loop {
            tokio::select! {
                _ = self.signal.notified() => {
                    debug!("Shutdown signal received, exiting server loop");
                    return Ok(());
                }

                connection = server.accept() => {
                    self.accept_connection(connection).await?;
                }
            }
        }
    }

    async fn accept_connection(
        &self,
        connection: Result<(TcpStream, SocketAddr), std::io::Error>,
    ) -> Result<(), ServerError> {
        let (stream, addr) = connection.map_err(ServerError::FailedToAcceptConnection)?;
        info!("Accepted connection from {}", addr);

        let _io = TokioIo::new(stream);

        // Handle the TCP stream in a separate task
        // tokio::spawn(async move {
        //     if let Err(e) = self.handle_tcp_stream(io).await {
        //         log::error!("Failed to handle TCP stream: {}", e);
        //     }
        // });
        //
        // Ok(())
        unimplemented!("Handling TCP stream is not implemented yet");
    }

    async fn handle_tcp_stream(&self, stream: TokioIo<TcpStream>) -> Result<(), ServerError> {
        unimplemented!("Handling TCP stream is not implemented yet");
    }

    /// Create the default TCP listener
    async fn make_tcp_listener(&self) -> Result<TcpListener, ServerError> {
        let socket_addr = self.get_socket_address()?;
        TcpListener::bind(socket_addr)
            .await
            .map_err(ServerError::FailedToBind)
    }
}
