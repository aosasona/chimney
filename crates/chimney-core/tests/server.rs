use chimney::{Server, config::Config, filesystem};

fn mock_config() -> Config {
    Config::default()
}

fn mock_server() -> Server {
    Server::new(
        chimney::config::LogLevel::Trace,
        filesystem::mock::new(),
        mock_config(),
    )
}

#[test]
pub fn test_get_socket_address() {}
