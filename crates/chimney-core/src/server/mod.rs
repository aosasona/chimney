pub mod mimetype;
pub mod service;

use std::{net::SocketAddr, sync::Arc};

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use log::{debug, error};

use crate::error::ServerError;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Notify, RwLock},
};

const SHUTDOWN_WAIT_PERIOD: u64 = 15; // seconds

pub struct Server {
    /// The configuration for the server
    config: Arc<RwLock<crate::config::Config>>,

    /// The shutdown signal for the server
    signal: Arc<Notify>,

    /// Whether to shut down gracefully (default: true)
    graceful_shutdown: bool,

    /// The service for handling requests
    service: service::Service,
}

impl Server {
    pub fn new(
        filesystem: Arc<dyn crate::filesystem::Filesystem>,
        config: Arc<RwLock<crate::config::Config>>,
    ) -> Self {
        debug!("Creating a new Chimney server instance");

        let service = service::Service::new(filesystem.clone(), config.clone());

        Server {
            config,
            signal: Arc::new(Notify::new()),
            graceful_shutdown: true,
            service,
        }
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

            debug!("Received shutdown signal, shutting down the server...");
            signal.notify_waiters();
        });
    }

    /// Get the socket address for the server based on the configuration.
    pub async fn get_socket_address(&self) -> Result<SocketAddr, ServerError> {
        let config = self.config.read().await;
        // Prevent the use of possibly reserved ports
        if config.port <= 1024 {
            return Err(ServerError::InvalidPortRange { port: config.port });
        }

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
        let listener = self.make_tcp_listener().await?;

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
                    self.handle_connection(connection).await?;
                }
            }
        }

        // Start graceful shutdown watcher when the main look is broken
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

    async fn handle_connection(
        &self,
        connection: Result<(TcpStream, SocketAddr), std::io::Error>,
    ) -> Result<(), ServerError> {
        let (stream, addr) = connection.map_err(ServerError::FailedToAcceptConnection)?;
        debug!("Accepted connection from {addr}");

        let io = TokioIo::new(stream);
        let service = self.service.clone();

        // Handle the TCP stream in a separate task
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                error!("Failed to serve connection: {err:?}");
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
