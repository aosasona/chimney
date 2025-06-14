#![allow(unused)] // TODO: remove
use std::{net::SocketAddr, sync::Arc};

use log::debug;

use crate::{config::LogLevel, error::ServerError};
use tokio::sync::Notify;

// TODO: build a domain and sites map to easily lookup sites by domain
pub struct Server<'a> {
    /// The filesystem abstraction used by the server
    filesystem: Box<dyn crate::filesystem::Filesystem>,

    /// The configuration for the server
    config: &'a mut crate::config::Config,

    /// The shutdown signal for the server
    signal: Arc<Notify>,

    /// Whether to shut down gracefully (default: true)
    graceful_shutdown: bool,
}

impl<'a> Server<'a> {
    pub fn new(
        filesystem: Box<dyn crate::filesystem::Filesystem>,
        config: &'a mut crate::config::Config,
    ) -> Self {
        debug!("Creating a new Chimney server instance");
        Server {
            filesystem,
            config,
            signal: Arc::new(Notify::new()),
            graceful_shutdown: true,
        }
    }

    /// Get the current configuration of the server.
    pub fn config(&self) -> &crate::config::Config {
        self.config
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

    pub async fn run(&self) -> Result<(), crate::error::ChimneyError> {
        // TODO: handle signal listening for graceful shutdown if enabled

        // Here you would implement the logic to start the server
        // For now, we just print the configuration and return Ok
        debug!(
            "Running in debug mode with configuration: {:?}",
            self.config
        );

        unimplemented!("Implement server logic here");
    }
}
