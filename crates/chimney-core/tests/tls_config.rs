use chimney::config::{Https, Site};
use chimney::tls::config::{process_site_https_config, TlsMode};
use std::collections::HashMap;

fn create_test_site(name: &str, domains: Vec<String>, https_config: Option<Https>) -> Site {
    Site {
        name: name.to_string(),
        domain_names: domains,
        https_config,
        root: ".".to_string(),
        fallback_file: None,
        default_index_file: Some("index.html".to_string()),
        response_headers: HashMap::new(),
        redirects: HashMap::new(),
        rewrites: HashMap::new(),
    }
}

#[test]
fn test_process_site_no_https_config_uses_acme() {
    // Sites without https_config use ACME by default
    let site = create_test_site("test", vec!["example.com".to_string()], None);
    let result = process_site_https_config(&site);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.site_name, "test");
    assert!(matches!(config.mode, TlsMode::Acme));
}

#[test]
fn test_process_site_https_config_acme() {
    // No cert_file/key_file means ACME mode
    let https = Https {
        auto_redirect: true,
        cert_file: None,
        key_file: None,
        ca_file: None,
    };

    let site = create_test_site("test", vec!["example.com".to_string()], Some(https));
    let result = process_site_https_config(&site);

    assert!(result.is_ok());
    let config = result.unwrap();

    assert_eq!(config.site_name, "test");
    assert_eq!(config.domains, vec!["example.com"]);
    assert!(matches!(config.mode, TlsMode::Acme));
}

#[test]
fn test_process_site_https_config_manual() {
    // Providing cert_file + key_file means manual mode
    let https = Https {
        auto_redirect: true,
        cert_file: Some("/path/to/cert.pem".to_string()),
        key_file: Some("/path/to/key.pem".to_string()),
        ca_file: Some("/path/to/ca.pem".to_string()),
    };

    let site = create_test_site("test", vec!["example.com".to_string()], Some(https));
    let result = process_site_https_config(&site);

    assert!(result.is_ok());
    let config = result.unwrap();

    assert_eq!(config.site_name, "test");
    assert!(matches!(config.mode, TlsMode::Manual { .. }));

    if let TlsMode::Manual {
        cert_file,
        key_file,
        ca_file,
    } = config.mode
    {
        assert_eq!(cert_file, "/path/to/cert.pem");
        assert_eq!(key_file, "/path/to/key.pem");
        assert_eq!(ca_file, Some("/path/to/ca.pem".to_string()));
    }
}

#[test]
fn test_process_site_https_config_incomplete_manual() {
    // Only cert_file without key_file is an error
    let https = Https {
        auto_redirect: true,
        cert_file: Some("/path/to/cert.pem".to_string()),
        key_file: None,
        ca_file: None,
    };

    let site = create_test_site("test", vec!["example.com".to_string()], Some(https));
    let result = process_site_https_config(&site);

    assert!(result.is_err());
}

#[test]
fn test_process_site_https_config_multiple_domains() {
    let domains = vec![
        "example.com".to_string(),
        "www.example.com".to_string(),
        "api.example.com".to_string(),
    ];

    let site = create_test_site("test", domains.clone(), None);
    let result = process_site_https_config(&site);

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.domains, domains);
    assert!(matches!(config.mode, TlsMode::Acme));
}
