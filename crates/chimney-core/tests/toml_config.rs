use chimney_core::config::{Format, toml::Toml};

#[test]
pub fn parse_root_config() {
    let input = r#"
    host = "0.0.0.0"
    port = 80
    sites_directory = ["./sites"]
    log_level = "debug"
    "#;

    let toml_parser = Toml::new(input);
    let config = toml_parser.parse().expect("Failed to parse TOML config");

    assert_eq!(config.host.to_string(), "0.0.0.0");
    assert_eq!(config.port, 80);
    assert_eq!(config.sites_directory, vec!["./sites"]);
    assert_eq!(config.log_level.to_string(), "debug");
    assert!(config.sites.is_empty(), "Expected no sites in the config");
}
#[test]
pub fn parse_empty_root_config() {
    let input = "";

    let toml_parser = Toml::new(input);
    let config = toml_parser
        .parse()
        .expect("Failed to parse empty TOML config");

    assert_eq!(config.host.to_string(), "0.0.0.0");
    assert_eq!(config.port, 8080);
    assert_eq!(
        config.sites_directory,
        vec![
            std::env::current_dir()
                .unwrap()
                .join("sites")
                .to_string_lossy()
                .to_string()
        ]
    );
    assert_eq!(config.log_level.to_string(), "info");
    assert!(config.sites.is_empty(), "Expected no sites in the config");
}

#[test]
pub fn parse_partial_root_config() {
    let input = r#"
    host = "0.0.0.0"
    log_level = "warn"
    "#;

    let toml_parser = Toml::new(input);
    let config = toml_parser
        .parse()
        .expect("Failed to parse partial TOML config");

    assert_eq!(config.host.to_string(), "0.0.0.0");
    assert_eq!(config.port, 8080);
    assert_eq!(
        config.sites_directory,
        vec![
            std::env::current_dir()
                .unwrap()
                .join("sites")
                .to_string_lossy()
                .to_string()
        ]
    );
    assert_eq!(config.log_level.to_string(), "warn");
    assert!(config.sites.is_empty(), "Expected no sites in the config");
}

// TODO: add tests for sites parsing
