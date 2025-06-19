#![allow(unused)]
pub mod resolver;

// TODO: remove
use std::{net::SocketAddr, sync::Arc};

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use log::{debug, info};

use crate::error::ServerError;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Notify, RwLock},
};

const SHUTDOWN_WAIT_PERIOD: u64 = 15; // seconds

// TODO: build a domain and sites map to easily lookup sites by domain
pub struct Server {
    /// The filesystem abstraction used by the server
    filesystem: Arc<dyn crate::filesystem::Filesystem>,

    /// The configuration for the server
    config: Arc<RwLock<crate::config::Config>>,

    /// The shutdown signal for the server
    signal: Arc<Notify>,

    /// Whether to shut down gracefully (default: true)
    graceful_shutdown: bool,

    /// The resolver for handling path and resource resolution
    resolver: resolver::Resolver,
}

impl Server {
    pub fn new(
        filesystem: Arc<dyn crate::filesystem::Filesystem>,
        config: Arc<RwLock<crate::config::Config>>,
    ) -> Self {
        debug!("Creating a new Chimney server instance");

        let resolver = resolver::Resolver::new(filesystem.clone(), config.clone());

        Server {
            filesystem,
            config,
            signal: Arc::new(Notify::new()),
            graceful_shutdown: true,
            resolver,
        }
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
    pub async fn get_socket_address(&self) -> Result<SocketAddr, ServerError> {
        let config = self.config.read().await;
        // Prevent the use of possibly reserved ports
        if config.port <= 1024 {
            return Err(ServerError::InvalidPortRange { port: config.port });
        }

        let raw_addr = format!("{}:{}", config.host, config.port);
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
                    self.accept_connection(connection).await?;
                }
            }
        }

        // Start graceful shutdown watcher when the main look is broken
        tokio::select! {
            _ = graceful.shutdown() => {
                debug!("Graceful shutdown initiated, exiting server loop");
                Ok(())
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(SHUTDOWN_WAIT_PERIOD)) => {
                log::error!("timed out wait for all connections to close");
                Err(ServerError::TimeoutWaitingForConnections)
            }
        }
    }

    async fn accept_connection(
        &self,
        connection: Result<(TcpStream, SocketAddr), std::io::Error>,
    ) -> Result<(), ServerError> {
        let (stream, addr) = connection.map_err(ServerError::FailedToAcceptConnection)?;
        info!("Accepted connection from {}", addr);

        let io = TokioIo::new(stream);
        let resolver = self.resolver.clone();

        // Handle the TCP stream in a separate task
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(io, resolver).await {
                println!("Failed to serve connection: {:?}", err);
            }
        });

        Ok(())
    }

    async fn handle_tcp_stream(
        &self,
        _addr: SocketAddr,
        _stream: TokioIo<TcpStream>,
    ) -> Result<(), ServerError> {
        unimplemented!("Handling TCP stream is not implemented yet");
    }

    /// Create the default TCP listener
    async fn make_tcp_listener(&self) -> Result<TcpListener, ServerError> {
        let socket_addr = self.get_socket_address().await?;
        TcpListener::bind(socket_addr)
            .await
            .map_err(ServerError::FailedToBind)
    }
}
