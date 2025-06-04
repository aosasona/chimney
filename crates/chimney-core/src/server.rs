pub struct Server {
    /// Whether to run the server in debug mode
    debug: bool,

    /// The filesystem abstraction used by the server
    filesystem: Box<dyn crate::filesystem::Filesystem>,

    /// The configuration for the server
    config: crate::config::Config,
}

impl Server {
    pub fn new(fs: Box<dyn crate::filesystem::Filesystem>, config: crate::config::Config) -> Self {
        Self {
            debug: false, // Default to false; can be set later if needed
            filesystem: fs,
            config,
        }
    }

    /// Sets the debug mode for the server
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }
}
