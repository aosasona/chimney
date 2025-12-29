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
    fallback_file = "index.html"
    https_config = { cert_file = "tls/cert.pem", key_file = "tls/key.pem" }
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
    assert_eq!(site.fallback_file, Some("index.html".to_string()));
    assert!(
        site.https_config.is_some(),
        "Expected HTTPS config to be present"
    );

    let https_config = site.https_config.as_ref().unwrap();
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
    fallback_file = "index.html"
    https_config = { cert_file = "tls/cert.pem", key_file = "tls/key.pem" }
    "#;

    let site = Site::from_string(name.into(), input)
        .expect("Failed to parse standalone site config with manual HTTPS");

    assert_eq!(site.name, "example");
    assert_eq!(site.root, "./public");
    assert_eq!(site.domain_names, vec!["example.com"]);
    assert_eq!(site.fallback_file, Some("index.html".to_string()));
    assert!(
        site.https_config.is_some(),
        "Expected HTTPS config to be present"
    );

    let https_config = site.https_config.as_ref().unwrap();
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
    # with temporary flag
    "/foo" = { to = "/new-path", temporary = true }
    # without temporary flag
    "/bar" = "/another-new-path"
    # with replay and temporary redirect
    "/baz" = { to = "/yet-another-path", temporary = true, replay = true }
    "#;

    let site = Site::from_string(name.into(), input)
        .expect("Failed to parse standalone site config with manual HTTPS");

    assert_eq!(site.name, "example");
    assert_eq!(site.root, "./public");

    assert_eq!(
        site.redirects.len(),
        3,
        "Expected two redirects in the site config"
    );

    // With temporary redirect
    let redirect_foo = site
        .find_redirect_rule("/foo")
        .expect("Redirect for '/foo' not found");
    assert_eq!(
        redirect_foo.target(),
        "/new-path",
        "Expected redirect '/foo' to point to '/new-path'"
    );
    assert!(
        redirect_foo.is_temporary(),
        "Expected redirect '/foo' to have replay enabled"
    );
    assert!(
        !redirect_foo.is_replay(),
        "Expected redirect '/foo' to not have replay enabled"
    );

    // Without temporary redirect
    let redirect_bar = site
        .find_redirect_rule("/bar")
        .expect("Redirect for '/bar' not found");
    assert_eq!(
        redirect_bar.target(),
        "/another-new-path",
        "Expected redirect '/bar' to point to '/another-new-path'"
    );
    // This redirect should not have the temporary flag set
    assert!(
        !redirect_bar.is_temporary(),
        "Expected redirect '/bar' to not have temporary flag enabled"
    );
    // This redirect should not have replay enabled
    assert!(
        !redirect_bar.is_replay(),
        "Expected redirect '/bar' to not have replay enabled"
    );

    assert!(
        !redirect_bar.is_temporary(),
        "Expected redirect '/bar' to not have replay enabled"
    );

    let redirect_baz = site
        .find_redirect_rule("/baz")
        .expect("Redirect for '/baz' not found");
    assert_eq!(
        redirect_baz.target(),
        "/yet-another-path",
        "Expected redirect '/baz' to point to '/yet-another-path'"
    );
    assert!(
        redirect_baz.is_temporary(),
        "Expected redirect '/baz' to have temporary flag enabled"
    );
    assert!(
        redirect_baz.is_replay(),
        "Expected redirect '/baz' to have replay enabled"
    );
}

#[test]
pub fn parse_site_config_with_rewrite() {
    let name = "example";
    let input = r#"
    root = "./public"
    domain_names = ["example.com"]

    [rewrites]
    "/foo" = "/foo-rewrite"
    "/bar" = "/bar-rewrite"
    "#;

    let site = Site::from_string(name.into(), input)
        .expect("Failed to parse standalone site config with manual HTTPS");

    assert_eq!(site.name, "example");
    assert_eq!(site.root, "./public");

    assert_eq!(
        site.rewrites.len(),
        2,
        "Expected two rewrites in the site config"
    );

    // Check rewrite for "/foo"
    let rewrite_foo = site
        .find_rewrite_rule("/foo")
        .expect("Rewrite for '/foo' not found");
    assert_eq!(
        rewrite_foo.target(),
        "/foo-rewrite",
        "Expected rewrite '/foo' to point to '/foo-rewrite'"
    );

    // Check rewrite for "/bar"
    let rewrite_bar = site
        .find_rewrite_rule("/bar")
        .expect("Rewrite for '/bar' not found");

    assert_eq!(
        rewrite_bar.target(),
        "/bar-rewrite",
        "Expected rewrite '/bar' to point to '/bar-rewrite'"
    );

    // Check for a fake rewrite
    let rewrite_baz = site.find_rewrite_rule("/baz");
    assert!(
        rewrite_baz.is_none(),
        "Expected no rewrite for '/baz', but found one"
    );
}
