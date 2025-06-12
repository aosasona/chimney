#![allow(unused)] // TODO: remove
use std::{net::SocketAddr, sync::Arc};

use log::debug;

use crate::{config::LogLevel, error::ServerError};
use tokio::sync::Notify;

// TODO: build a domain and sites map to easily lookup sites by domain
pub struct Server {
    /// The current global log level (this could be from a CLI argument or environment variable)
    global_log_level: LogLevel,

    /// The filesystem abstraction used by the server
    filesystem: Box<dyn crate::filesystem::Filesystem>,

    /// The configuration for the server
    config: crate::config::Config,

    /// The shutdown signal for the server
    signal: Arc<Notify>,
}

impl Server {
    pub fn new(
        global_log_level: LogLevel,
        filesystem: Box<dyn crate::filesystem::Filesystem>,
        config: crate::config::Config,
    ) -> Self {
        debug!("Creating a new Chimney server instance");
        Server {
            global_log_level,
            filesystem,
            config,
            signal: Arc::new(Notify::new()),
        }
    }

    /// Get the current configuration of the server.
    pub fn config(&self) -> &crate::config::Config {
        &self.config
    }

    /// Watch for a shutdown signal (like Ctrl+C) and notify the server to shut down gracefully.
    async fn watch_for_shutdown(&self) {
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
        // Here you would implement the logic to start the server
        // For now, we just print the configuration and return Ok
        debug!(
            "Running in debug mode with configuration: {:?}",
            self.config
        );

        unimplemented!("Implement server logic here");
    }
}
