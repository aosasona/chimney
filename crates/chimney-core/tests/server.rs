use std::sync::Arc;

use chimney::{
    config::{Config, Format, toml},
    filesystem,
    server::Server,
};
use tokio::sync::RwLock;

macro_rules! test_socket_address {
    ($config:expr) => {
        let config_clone = $config.clone();
        let server = mock_server($config);
        let addr = server.get_socket_address().await;
        assert!(
            addr.is_ok(),
            "Failed to get socket address: {:?}",
            addr.err()
        );

        let socket_addr = addr.unwrap();
        let exptected_addr = format!("{}:{}", config_clone.host, config_clone.port);
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
        let addr = server.get_socket_address().await;
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
    let fs = Arc::new(filesystem::mock::MockFilesystem);
    let config = Arc::new(RwLock::new(config));
    Server::new(fs, config)
}

#[tokio::test]
// Test with the mock server configuration
pub async fn test_get_socket_address_with_default_config() {
    let config = mock_config();
    test_socket_address!(config);
}

#[tokio::test]
// Test with a custom configuration
pub async fn test_get_socket_address_with_custom_config() {
    let config = config!(
        r#"
        host = "192.168.0.1"
        port = 8080
        sites_directory = "./sites"
        "#
    );
    test_socket_address!(config);
}

#[tokio::test]
// Test with a configuration that has a reserved port
pub async fn test_get_socket_address_with_invalid_config() {
    let config = config!(
        r#"
            host = "0.0.0.0"
            port = 80
            sites_directory = "./sites"
            "#
    );
    test_socket_address!(config, reserved);
}
