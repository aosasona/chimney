use chimney::{
    config::{Config, ConfigHandle, HostDetectionStrategy},
    filesystem::mock::MockFilesystem,
    server::service::Service,
};
use hyper::{
    header::{HeaderValue, HOST, HeaderName},
    HeaderMap,
};
use std::sync::Arc;

fn create_test_config(host_detection: HostDetectionStrategy) -> Config {
    let mut config = Config::default();
    config.host_detection = host_detection;
    config
}

fn create_config_handle(config: Config) -> ConfigHandle {
    let (tx, rx) = tokio::sync::watch::channel(Arc::new(config));
    ConfigHandle::new(tx, rx)
}

fn create_service(config: Config) -> Service {
    let fs = Arc::new(MockFilesystem);
    let config_handle = create_config_handle(config);
    Service::new(fs, config_handle)
}

#[tokio::test]
async fn test_auto_host_detection_with_host_header() {
    let config = create_test_config(HostDetectionStrategy::Auto);
    let service = create_service(config);

    let mut headers = HeaderMap::new();
    headers.insert(HOST, HeaderValue::from_static("example.com"));

    let result = service.resolve_host(&headers).await;
    assert!(result.is_ok());

    let resolved = result.unwrap();
    assert_eq!(resolved.host, "example.com");
    assert!(resolved.is_auto);
    // Header names are normalized to lowercase
    assert_eq!(resolved.header.to_lowercase(), "host");
}

#[tokio::test]
async fn test_auto_host_detection_with_x_forwarded_host() {
    let config = create_test_config(HostDetectionStrategy::Auto);
    let service = create_service(config);

    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-forwarded-host"),
        HeaderValue::from_static("proxy.example.com"),
    );

    let result = service.resolve_host(&headers).await;
    assert!(result.is_ok());

    let resolved = result.unwrap();
    assert_eq!(resolved.host, "proxy.example.com");
    assert!(resolved.is_auto);
    assert_eq!(resolved.header.to_lowercase(), "x-forwarded-host");
}

#[tokio::test]
async fn test_auto_host_detection_prefers_host_over_x_forwarded_host() {
    let config = create_test_config(HostDetectionStrategy::Auto);
    let service = create_service(config);

    let mut headers = HeaderMap::new();
    headers.insert(HOST, HeaderValue::from_static("example.com"));
    headers.insert(
        HeaderName::from_static("x-forwarded-host"),
        HeaderValue::from_static("proxy.example.com"),
    );

    let result = service.resolve_host(&headers).await;
    assert!(result.is_ok());

    let resolved = result.unwrap();
    // Auto mode checks Host first, then X-Forwarded-Host
    assert_eq!(resolved.host, "example.com");
    assert_eq!(resolved.header.to_lowercase(), "host");
}

#[tokio::test]
async fn test_manual_host_detection_with_custom_headers() {
    let config = create_test_config(HostDetectionStrategy::Manual {
        target_headers: vec!["X-Custom-Host".to_string(), "Host".to_string()],
    });
    let service = create_service(config);

    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-custom-host"),
        HeaderValue::from_static("custom.example.com"),
    );
    headers.insert(HOST, HeaderValue::from_static("example.com"));

    let result = service.resolve_host(&headers).await;
    assert!(result.is_ok());

    let resolved = result.unwrap();
    // Should prefer X-Custom-Host as it's first in the list
    assert_eq!(resolved.host, "custom.example.com");
    assert!(!resolved.is_auto);
    assert_eq!(resolved.header.to_lowercase(), "x-custom-host");
}

#[tokio::test]
async fn test_manual_host_detection_fallback() {
    let config = create_test_config(HostDetectionStrategy::Manual {
        target_headers: vec!["X-Custom-Host".to_string(), "Host".to_string()],
    });
    let service = create_service(config);

    let mut headers = HeaderMap::new();
    // Only Host header present, X-Custom-Host missing
    headers.insert(HOST, HeaderValue::from_static("example.com"));

    let result = service.resolve_host(&headers).await;
    assert!(result.is_ok());

    let resolved = result.unwrap();
    // Should fall back to Host header
    assert_eq!(resolved.host, "example.com");
    assert_eq!(resolved.header.to_lowercase(), "host");
}

#[tokio::test]
async fn test_host_detection_with_port() {
    let config = create_test_config(HostDetectionStrategy::Auto);
    let service = create_service(config);

    let mut headers = HeaderMap::new();
    headers.insert(HOST, HeaderValue::from_static("example.com:8080"));

    let result = service.resolve_host(&headers).await;
    assert!(result.is_ok());

    let resolved = result.unwrap();
    // Should preserve port in hostname
    assert_eq!(resolved.host, "example.com:8080");
}

#[tokio::test]
async fn test_host_detection_failure_no_headers() {
    let config = create_test_config(HostDetectionStrategy::Manual {
        target_headers: vec!["X-Custom-Host".to_string()],
    });
    let service = create_service(config);

    let headers = HeaderMap::new(); // No headers

    let result = service.resolve_host(&headers).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_host_detection_invalid_utf8() {
    let config = create_test_config(HostDetectionStrategy::Auto);
    let service = create_service(config);

    let mut headers = HeaderMap::new();
    // Create invalid UTF-8 header value
    let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
    headers.insert(HOST, HeaderValue::from_bytes(&invalid_bytes).unwrap());

    let result = service.resolve_host(&headers).await;
    // Should fail gracefully with invalid UTF-8
    assert!(result.is_err());
}

#[tokio::test]
async fn test_host_detection_case_insensitive() {
    let config = create_test_config(HostDetectionStrategy::Manual {
        target_headers: vec!["Host".to_string()],
    });
    let service = create_service(config);

    let mut headers = HeaderMap::new();
    // HTTP headers are case-insensitive
    headers.insert(HOST, HeaderValue::from_static("Example.COM"));

    let result = service.resolve_host(&headers).await;
    assert!(result.is_ok());

    let resolved = result.unwrap();
    // Should preserve the case as received
    assert_eq!(resolved.host, "Example.COM");
}
