pub struct Server {
    /// Whether to run the server in debug mode
    debug: bool,

    /// The filesystem abstraction used by the server
    filesystem: Box<dyn crate::filesystem::Filesystem>,

    /// The configuration for the server
    config: crate::config::Config,
}

macro_rules! init_tracing {
    ($debug:expr) => {
        let log_level = if $debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        };

        let _ = tracing_subscriber::fmt()
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_level(true)
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
            .with_max_level(log_level)
            .try_init();
    };
}

impl Server {
    pub fn new(
        debug: bool,
        filesystem: Box<dyn crate::filesystem::Filesystem>,
        config: crate::config::Config,
    ) -> Self {
        // TODO: create an instance-level tracing subscriber instead of a global one here
        // TODO: remove `tracng_subscriber` dependency as recommended by the docs

        // Initialize tracing for the server
        init_tracing!(debug);

        Server {
            debug,
            filesystem,
            config,
        }
    }

    pub async fn run(&self) -> Result<(), crate::error::ChimneyError> {
        // Here you would implement the logic to start the server
        // For now, we just print the configuration and return Ok
        if self.debug {
            dbg!(&self.config);
            println!("Running in debug mode");
        }

        unimplemented!("Implement server logic here");
    }
}
