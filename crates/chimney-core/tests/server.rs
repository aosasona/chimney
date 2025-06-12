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
pub fn test_get_socket_address() {
    let server = mock_server();
    let addr = server.get_socket_address();
    assert!(
        addr.is_ok(),
        "Failed to get socket address: {:?}",
        addr.err()
    );

    let socket_addr = addr.unwrap();
    let exptected_addr = format!("{}:{}", server.config().host, server.config().port);
    assert_eq!(
        socket_addr.to_string(),
        exptected_addr,
        "Socket address does not match expected: {} != {}",
        socket_addr,
        exptected_addr
    );
}
