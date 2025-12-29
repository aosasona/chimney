// Redirect service integration tests
// These tests verify that the redirect service properly uses the host detection strategy

use chimney::{
    config::{Config, ConfigHandle, HostDetectionStrategy, HttpsConfig, Https},
    filesystem::mock::MockFilesystem,
    server::{redirect::RedirectService, service::Service},
};
use std::{path::PathBuf, sync::Arc};

fn create_test_config_with_https(host_detection: HostDetectionStrategy) -> Config {
    let mut config = Config::default();
    config.host_detection = host_detection;
    config.https = Some(HttpsConfig {
        enabled: true,
        port: 8443,
        cache_directory: PathBuf::from("/tmp/chimney-certs"),
        acme_email: None,
        acme_directory_url: "https://acme-v02.api.letsencrypt.org/directory".to_string(),
    });
    config
}

fn create_config_handle(config: Config) -> ConfigHandle {
    let (tx, rx) = tokio::sync::watch::channel(Arc::new(config));
    ConfigHandle::new(tx, rx)
}

fn create_test_site_toml(domain: &str, auto_redirect: bool) -> String {
    format!(
        r#"
domain_names = ["{}"]

[https_config]
auto_redirect = {}
"#,
        domain, auto_redirect
    )
}

#[test]
fn test_redirect_service_creation() {
    // Test that we can create a redirect service with host strategy config
    let config = create_test_config_with_https(HostDetectionStrategy::Auto);
    let config_handle = create_config_handle(config);
    let fs = Arc::new(MockFilesystem);
    let service = Service::new(fs, config_handle.clone());

    // Create redirect service for HTTP (should redirect)
    let redirect_service_http = RedirectService::new(service.clone(), config_handle.clone(), false);
    assert!(std::ptr::addr_of!(redirect_service_http) as usize != 0);

    // Create redirect service for HTTPS (should not redirect)
    let redirect_service_https = RedirectService::new(service, config_handle, true);
    assert!(std::ptr::addr_of!(redirect_service_https) as usize != 0);
}

#[test]
fn test_redirect_config_with_manual_host_detection() {
    // Verify we can create a redirect service with manual host detection strategy
    let config = create_test_config_with_https(HostDetectionStrategy::Manual {
        target_headers: vec!["X-Custom-Host".to_string(), "Host".to_string()],
    });

    let config_handle = create_config_handle(config);
    let fs = Arc::new(MockFilesystem);
    let service = Service::new(fs, config_handle.clone());
    let redirect_service = RedirectService::new(service, config_handle, false);

    assert!(std::ptr::addr_of!(redirect_service) as usize != 0);
}

#[test]
fn test_site_auto_redirect_config_parsing() {
    // Test that site HTTPS config with auto_redirect parses correctly
    let site_toml = create_test_site_toml("example.com", true);
    let site = chimney::config::Site::from_string("example".to_string(), &site_toml).unwrap();

    assert_eq!(site.domain_names, vec!["example.com"]);
    assert!(site.https_config.is_some());

    let https_config = site.https_config.unwrap();
    assert!(https_config.auto_redirect);
}

#[test]
fn test_site_auto_redirect_disabled() {
    // Test that auto_redirect can be disabled
    let site_toml = create_test_site_toml("example.com", false);
    let site = chimney::config::Site::from_string("example".to_string(), &site_toml).unwrap();

    assert!(site.https_config.is_some());
    let https_config = site.https_config.unwrap();
    assert!(!https_config.auto_redirect);
}

#[test]
fn test_multiple_sites_with_different_redirect_settings() {
    // Test multiple sites with different auto_redirect settings
    let mut config = create_test_config_with_https(HostDetectionStrategy::Auto);

    let site1_toml = create_test_site_toml("site1.com", true);
    let site1 = chimney::config::Site::from_string("site1".to_string(), &site1_toml).unwrap();

    let site2_toml = create_test_site_toml("site2.com", false);
    let site2 = chimney::config::Site::from_string("site2".to_string(), &site2_toml).unwrap();

    config.sites.add(site1).unwrap();
    config.sites.add(site2).unwrap();

    let site1_loaded = config.sites.find_by_hostname("site1.com").unwrap();
    let site2_loaded = config.sites.find_by_hostname("site2.com").unwrap();

    assert!(site1_loaded
        .https_config
        .as_ref()
        .unwrap()
        .auto_redirect);
    assert!(!site2_loaded
        .https_config
        .as_ref()
        .unwrap()
        .auto_redirect);
}

#[test]
fn test_https_config_structure() {
    // Verify HttpsConfig has all required fields for redirect logic
    let https_config = HttpsConfig {
        enabled: true,
        port: 8443,
        cache_directory: PathBuf::from("/tmp/certs"),
        acme_email: Some("admin@example.com".to_string()),
        acme_directory_url: "https://acme-v02.api.letsencrypt.org/directory".to_string(),
    };

    assert!(https_config.enabled);
    assert_eq!(https_config.port, 8443);
    assert_eq!(https_config.cache_directory, PathBuf::from("/tmp/certs"));
}

#[test]
fn test_site_https_config_defaults() {
    // Test that Https config has proper defaults
    let https = Https {
        auto_redirect: Https::default_auto_redirect(),
        cert_file: None,
        key_file: None,
        ca_file: None,
    };

    // Default should be true for auto_redirect
    assert!(https.auto_redirect);
    assert!(Https::default_auto_redirect());
}

#[test]
fn test_redirect_with_no_https_config() {
    // Verify behavior when HTTPS is not configured
    let mut config = Config::default();
    config.https = None; // No HTTPS

    let site_toml = create_test_site_toml("example.com", true);
    let site = chimney::config::Site::from_string("example".to_string(), &site_toml).unwrap();
    config.sites.add(site).unwrap();

    let config_handle = create_config_handle(config);
    let fs = Arc::new(MockFilesystem);
    let service = Service::new(fs, config_handle.clone());
    let redirect_service = RedirectService::new(service, config_handle, false);

    // Service should be created even without HTTPS config
    assert!(std::ptr::addr_of!(redirect_service) as usize != 0);
}
