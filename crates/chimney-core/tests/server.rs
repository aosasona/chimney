use chimney::{
    Server,
    config::{Config, Format, toml},
    filesystem,
};

macro_rules! test_socket_address {
    ($config:expr) => {
        let server = mock_server($config);
        let addr = server.get_socket_address();
        assert!(
            addr.is_ok(),
            "Failed to get socket address: {:?}",
            addr.err()
        );

        let socket_addr = addr.unwrap();
        let exptected_addr = format!("{}:{}", $config.host, $config.port);
        assert_eq!(
            socket_addr.to_string(),
            exptected_addr,
            "Socket address does not match expected: {} != {}",
            socket_addr,
            exptected_addr
        );
    };
    ($config:expr, reserved) => {
        let server = mock_server($config);
        let addr = server.get_socket_address();
        assert!(
            addr.is_err(),
            "Expected an error for config, but got: {:?}",
            addr
        );
    };
}

macro_rules! config {
    ($input:expr) => {{
        let toml_config = toml::Toml::from($input);
        toml_config.parse().expect("Failed to parse TOML config")
    }};
    () => {
        mock_config()
    };
}

fn mock_config() -> Config {
    Config::default()
}

fn mock_server(config: Config) -> Server {
    Server::new(
        chimney::config::LogLevel::Trace,
        filesystem::mock::new(),
        config,
    )
}

#[test]
// Test with the mock server configuration
pub fn test_get_socket_address_with_default_config() {
    test_socket_address!(mock_config());
}

#[test]
// Test with a custom configuration
pub fn test_get_socket_address_with_custom_config() {
    test_socket_address!(config!(
        r#"
        host = "192.168.0.1"
        port = 8080
        sites_directory = "./sites"
        "#
    ));
}

#[test]
// Test with a configuration that has a reserved port
pub fn test_get_socket_address_with_invalid_config() {
    test_socket_address!(
        config!(
            r#"
            host = "0.0.0.0"
            port = 80
            sites_directory = "./sites"
            "#
        ),
        reserved
    );
}
