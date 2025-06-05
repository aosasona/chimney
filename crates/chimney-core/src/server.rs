use log::debug;

use crate::config::LogLevel;

pub struct Server {
    /// The current global log level (this could be from a CLI argument or environment variable)
    global_log_level: LogLevel,

    /// The filesystem abstraction used by the server
    filesystem: Box<dyn crate::filesystem::Filesystem>,

    /// The configuration for the server
    config: crate::config::Config,
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
        }
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
