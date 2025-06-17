use std::sync::Arc;

use log::debug;

/// A resolver handles the resolution of paths and resources in the Chimney server.
pub struct Resolver {
    /// The filesystem abstraction used by the server
    filesystem: Arc<dyn crate::filesystem::Filesystem>,

    /// The configuration for the server
    config: Arc<crate::config::Config>,
}

impl Resolver {
    pub fn new(
        filesystem: Arc<dyn crate::filesystem::Filesystem>,
        config: Arc<crate::config::Config>,
    ) -> Self {
        debug!("Creating a new Resolver instance");
        Resolver { filesystem, config }
    }
}
