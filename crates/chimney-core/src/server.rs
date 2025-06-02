pub struct Server {
    filesystem: Box<dyn crate::filesystem::Filesystem>,
    config: crate::config::Config,
}
