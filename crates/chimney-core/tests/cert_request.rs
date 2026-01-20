use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;

use chimney::error::ServerError;
use chimney::tls::{
    request_certificate, CertRequestOptions, CertRequestOptionsBuilder,
    LETS_ENCRYPT_PRODUCTION_URL, LETS_ENCRYPT_STAGING_URL,
};

#[test]
fn test_cert_request_options_default() {
    let options = CertRequestOptions::default();
    assert!(options.domains.is_empty());
    assert!(options.email.is_empty());
    assert_eq!(options.challenge_port, 443);
    assert_eq!(options.directory_url, LETS_ENCRYPT_PRODUCTION_URL);
    assert_eq!(options.timeout, Duration::from_secs(300));
}

#[tokio::test]
async fn test_request_certificate_validates_empty_domains() {
    let options = CertRequestOptions {
        domains: vec![],
        email: "test@example.com".to_string(),
        ..Default::default()
    };

    let result = request_certificate(options).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ServerError::TlsInitializationFailed(_)));
}

#[tokio::test]
async fn test_request_certificate_validates_empty_email() {
    let options = CertRequestOptions {
        domains: vec!["example.com".to_string()],
        email: String::new(),
        ..Default::default()
    };

    let result = request_certificate(options).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ServerError::TlsInitializationFailed(_)));
}

#[test]
fn test_lets_encrypt_urls() {
    assert!(LETS_ENCRYPT_PRODUCTION_URL.contains("acme-v02"));
    assert!(LETS_ENCRYPT_STAGING_URL.contains("staging"));
}

#[test]
fn test_cert_request_options_builder_basic() {
    let options = CertRequestOptionsBuilder::new()
        .domain("example.com")
        .email("admin@example.com")
        .build();

    assert_eq!(options.domains, vec!["example.com"]);
    assert_eq!(options.email, "admin@example.com");
    assert_eq!(options.directory_url, LETS_ENCRYPT_PRODUCTION_URL);
}

#[test]
fn test_cert_request_options_builder_multiple_domains() {
    let options = CertRequestOptionsBuilder::new()
        .domain("example.com")
        .domain("www.example.com")
        .domains(["api.example.com", "admin.example.com"])
        .email("admin@example.com")
        .build();

    assert_eq!(options.domains.len(), 4);
    assert!(options.domains.contains(&"example.com".to_string()));
    assert!(options.domains.contains(&"www.example.com".to_string()));
    assert!(options.domains.contains(&"api.example.com".to_string()));
    assert!(options.domains.contains(&"admin.example.com".to_string()));
}

#[test]
fn test_cert_request_options_builder_staging() {
    let options = CertRequestOptionsBuilder::new()
        .domain("example.com")
        .email("admin@example.com")
        .staging()
        .build();

    assert_eq!(options.directory_url, LETS_ENCRYPT_STAGING_URL);
}

#[test]
fn test_cert_request_options_builder_production() {
    let options = CertRequestOptionsBuilder::new()
        .domain("example.com")
        .email("admin@example.com")
        .staging()
        .production() // Override staging
        .build();

    assert_eq!(options.directory_url, LETS_ENCRYPT_PRODUCTION_URL);
}

#[test]
fn test_cert_request_options_builder_all_options() {
    use std::net::Ipv4Addr;

    let options = CertRequestOptionsBuilder::new()
        .domain("example.com")
        .email("admin@example.com")
        .cache_dir("/custom/certs")
        .challenge_port(8443)
        .timeout(Duration::from_secs(600))
        .bind_host(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
        .build();

    assert_eq!(options.cache_dir, PathBuf::from("/custom/certs"));
    assert_eq!(options.challenge_port, 8443);
    assert_eq!(options.timeout, Duration::from_secs(600));
    assert_eq!(options.bind_host, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
}
