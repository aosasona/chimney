use chimney::config::{Format, HostDetectionStrategy, LogLevel, Site, toml::Toml};

#[test]
pub fn parse_root_config() {
    let input = r#"
    host = "0.0.0.0"
    port = 80
    sites_directory = "./sites"
    log_level = "debug"
    "#;

    let toml_parser = Toml::new(input);
    let config = toml_parser.parse().expect("Failed to parse TOML config");

    assert_eq!(config.host.to_string(), "0.0.0.0");
    assert_eq!(config.port, 80);
    assert_eq!(config.sites_directory, "./sites");
    assert_eq!(config.log_level, Some(LogLevel::Debug));
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
        std::env::current_dir()
            .unwrap()
            .join("sites")
            .to_string_lossy()
            .to_string()
    );
    assert_eq!(config.log_level, None);
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
        std::env::current_dir()
            .unwrap()
            .join("sites")
            .to_string_lossy()
            .to_string()
    );
    assert_eq!(config.log_level, Some(LogLevel::Warn));
    assert!(config.sites.is_empty(), "Expected no sites in the config");
}

#[test]
pub fn parse_embedded_site_config_with_manual_https() {
    let input = r#"
    host = "0.0.0.0"
    log_level = "warn"

    [sites.example]
    root = "./public"
    domain_names = ["example.com"]
    fallback = "index.html"
    https_config = { enabled = true, cert_file = "tls/cert.pem", key_file = "tls/key.pem" }
    "#;

    let toml_parser = Toml::new(input);
    let config = toml_parser
        .parse()
        .expect("Failed to parse TOML config with embedded site");

    assert_eq!(config.host.to_string(), "0.0.0.0");
    assert_eq!(config.port, 8080);
    assert_eq!(
        config.sites_directory,
        std::env::current_dir()
            .unwrap()
            .join("sites")
            .to_string_lossy()
            .to_string()
    );
    assert_eq!(config.log_level, Some(LogLevel::Warn));

    assert_eq!(config.sites.len(), 1, "Expected one site in the config");

    let site = config
        .sites
        .get("example")
        .expect("Site 'example' not found");

    assert_eq!(site.name, "example");
    assert_eq!(site.root, "./public");
    assert_eq!(site.domain_names, vec!["example.com"]);
    assert_eq!(site.fallback, Some("index.html".to_string()));
    assert!(
        site.https_config.is_some(),
        "Expected HTTPS config to be present"
    );

    let https_config = site.https_config.as_ref().unwrap();
    assert!(https_config.enabled, "HTTPS should be enabled");
    assert_eq!(https_config.cert_file, Some("tls/cert.pem".to_string()));
    assert_eq!(https_config.key_file, Some("tls/key.pem".to_string()));
    assert!(https_config.ca_file.is_none(), "CA file should not be set");
}

#[test]
pub fn parse_standalone_site_config_with_manual_https() {
    let name = "example";
    let input = r#"
    root = "./public"
    domain_names = ["example.com"]
    fallback = "index.html"
    https_config = { enabled = true, cert_file = "tls/cert.pem", key_file = "tls/key.pem" }
    "#;

    let site = Site::from_string(name.into(), input)
        .expect("Failed to parse standalone site config with manual HTTPS");

    assert_eq!(site.name, "example");
    assert_eq!(site.root, "./public");
    assert_eq!(site.domain_names, vec!["example.com"]);
    assert_eq!(site.fallback, Some("index.html".to_string()));
    assert!(
        site.https_config.is_some(),
        "Expected HTTPS config to be present"
    );

    let https_config = site.https_config.as_ref().unwrap();
    assert!(https_config.enabled, "HTTPS should be enabled");
    assert_eq!(https_config.cert_file, Some("tls/cert.pem".to_string()));
    assert_eq!(https_config.key_file, Some("tls/key.pem".to_string()));
    assert!(https_config.ca_file.is_none(), "CA file should not be set");
}

#[test]
pub fn parse_config_with_auto_host_detection() {
    let input = r#"
    host_detection = "auto"
    "#;

    let toml_parser = Toml::new(input);
    let config = toml_parser
        .parse()
        .expect("Failed to parse TOML config with embedded site");

    assert!(
        config.host_detection.is_auto(),
        "Expected auto host detection"
    );
    assert_eq!(config.host_detection, HostDetectionStrategy::Auto);
}

#[test]
pub fn parse_config_with_manual_host_detection() {
    let input = r#"
    host_detection = { strategy = "manual", target_headers = ["Host", "X-Forwarded-Host"] }
    "#;

    let toml_parser = Toml::new(input);
    let config = toml_parser
        .parse()
        .expect("Failed to parse TOML config with manual host detection");

    assert!(
        !config.host_detection.is_auto(),
        "Expected manual host detection"
    );
    assert_eq!(
        config.host_detection,
        HostDetectionStrategy::Manual {
            target_headers: vec!["Host".to_string(), "X-Forwarded-Host".to_string()]
        }
    );
}

#[test]
pub fn parse_site_config_with_redirect() {
    let name = "example";
    let input = r#"
    root = "./public"
    domain_names = ["example.com"]

    [redirects]
    # with replay flag
    "/foo" = { to = "/new-path", replay = true }
    # without replay flag
    "/bar" = "/another-new-path"
    "#;

    let site = Site::from_string(name.into(), input)
        .expect("Failed to parse standalone site config with manual HTTPS");

    assert_eq!(site.name, "example");
    assert_eq!(site.root, "./public");

    assert_eq!(
        site.redirects.len(),
        2,
        "Expected two redirects in the site config"
    );

    // With replay
    let redirect_foo = site
        .find_redirect_rule("/foo")
        .expect("Redirect for '/foo' not found");
    assert_eq!(
        redirect_foo.target(),
        "/new-path",
        "Expected redirect '/foo' to point to '/new-path'"
    );
    assert!(
        redirect_foo.is_replay(),
        "Expected redirect '/foo' to have replay enabled"
    );

    // Without replay
    let redirect_bar = site
        .find_redirect_rule("/bar")
        .expect("Redirect for '/bar' not found");
    assert_eq!(
        redirect_bar.target(),
        "/another-new-path",
        "Expected redirect '/bar' to point to '/another-new-path'"
    );

    assert!(
        !redirect_bar.is_replay(),
        "Expected redirect '/bar' to not have replay enabled"
    );

    let redirect_baz = site.find_redirect_rule("/baz");
    assert!(
        redirect_baz.is_none(),
        "Expected no redirect for '/baz', but found one"
    );
}
