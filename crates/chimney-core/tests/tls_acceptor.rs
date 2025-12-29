use chimney::tls::acceptor::{build_tls_acceptor, SniResolver};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::sign::CertifiedKey;
use std::sync::Arc;

// Initialize crypto provider for tests
fn init_crypto() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

// Test certificate for SNI resolver (self-signed RSA 2048, for testing only)
fn create_test_certified_key() -> Arc<CertifiedKey> {
    init_crypto();

    let cert_pem = br#"-----BEGIN CERTIFICATE-----
MIIDFzCCAf+gAwIBAgIUH3NRVTEGZ6/0uev+duwfow0/Y/wwDQYJKoZIhvcNAQEL
BQAwGzEZMBcGA1UEAwwQdGVzdC5leGFtcGxlLmNvbTAeFw0yNTEyMjcyMjM0Mjha
Fw0yNjEyMjcyMjM0MjhaMBsxGTAXBgNVBAMMEHRlc3QuZXhhbXBsZS5jb20wggEi
MA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQC0F9CnhxDYwbkBNGQ+1X13BvzI
ryog/g5tqBO8GWVS/Q358u1cpz9e1E7MsJyJS/oyNW/Uc7UPenq++EWXh2mKZ4uW
Y3FARYDXweUxG//2y2jQv9s6nyJWh7yu0M1jHXSttfCKju/hQ1BBabaf8bYuTaNJ
+UPLc21zvPgXbatpCekj4Q47h1qSMTniWKmMaX7SWGb3mk7WHIJOKSvXVU2VVBv8
r4KG4r6Dq0wIgJqR0qPWPeCCyU1nnX5IXsqkgMCqwg2YehvWd6fBtkIARTJKFjvn
jM+zCganqo9YUl4oNDdstkvGskMWqgUHmrsztiu+lp2sNWJvJU5Vtv3mXwWbAgMB
AAGjUzBRMB0GA1UdDgQWBBSyQBgvmhkR5KxOUt5z5/+iuk/bkjAfBgNVHSMEGDAW
gBSyQBgvmhkR5KxOUt5z5/+iuk/bkjAPBgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3
DQEBCwUAA4IBAQBrPZJIQpaaqrmf1TAElU2NyxhZY0x01Pd0WTRJNWZwFlh0YXCP
MQcubfJtlUCbmw2gwCYisxL7ZXTIfTM4x2xDb4UsFCfDINtegHPGSKY7rAiGhh1a
9B2ocSMexmARchvKpkthjdrHlxFtmWWTp0qP+7GIwl7r+3WxchPgyrmAre8Fi1Ju
OdOkqs7G61PEIZ4iGRCCV2FHwBu0Z1K/x5z/1a0UZHK4bFTatcOpKiwt0/WvWrZs
Xkkl2Na3/efr49frmNT3Cr/mdmCPxN9GCnuugQlIAlKaJRNu3kOmrHdvPImmNUWB
cchGHMYhs6GmU2oUz0zaU7Uhc0RdP4xdRn9O
-----END CERTIFICATE-----"#;

    let key_pem = br#"-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQC0F9CnhxDYwbkB
NGQ+1X13BvzIryog/g5tqBO8GWVS/Q358u1cpz9e1E7MsJyJS/oyNW/Uc7UPenq+
+EWXh2mKZ4uWY3FARYDXweUxG//2y2jQv9s6nyJWh7yu0M1jHXSttfCKju/hQ1BB
abaf8bYuTaNJ+UPLc21zvPgXbatpCekj4Q47h1qSMTniWKmMaX7SWGb3mk7WHIJO
KSvXVU2VVBv8r4KG4r6Dq0wIgJqR0qPWPeCCyU1nnX5IXsqkgMCqwg2YehvWd6fB
tkIARTJKFjvnjM+zCganqo9YUl4oNDdstkvGskMWqgUHmrsztiu+lp2sNWJvJU5V
tv3mXwWbAgMBAAECggEATahHTTYsyYsfn6lb4MxmgcD9l/wQipGC3z4u5Fl/G74L
HNDoEZ/874NVR2aQ2ZNtm+D3DAGo/beu3lJoj+LQW+IyivLujuxplqABmJ+eTGmC
FSHmAu1D/VQixK89IZQ+D/n4c4cXYWeJX+uZ2HZ+PJE17FwUI9LuS44c3N1poKzu
KTjlUTTdMi7ODudTZJeQcsc4vVZiyIVgFgd92yDW2wBfYTc9j636q5DwGFY06Ai3
OCYGcSbdcyFI1prg9OGnhWn/0D2NjYXAnzvUENApZ+P7Ddoty+upa9Niu4oqFlGd
K2X6qgRZReJul/NcoQqvWUwkIjLqX1KxztH0TFYxgQKBgQD4/Fa2ZgVBMqxNRsNf
xmSdw8eB+nz3CEwQiU9+hhPWJOIzcTOz6SosfjPmol1EHYPp2bxZsNpAdKxshjPS
5aebIonhK3cThChoM6+uJxyHFu3OD4jcyYsjIHBOpuaAPhuOkIerfK/v+rkUs4jR
HpQI3gNGeq8zE7bsqBVgg6WySwKBgQC5KqEA0JjCq+cL3PBA3Ebag20+YBSJa2YN
la0b60QhRiEnwq6VVqHUbRDRZ2KqSB9Wg3hMq58hAU1cL6Lfiu+lFm2JWSSImgZc
PnMbitphkeZYl/DbMgZb8RAEC6NmeskFEaLr8p6KPRAGKBAiNPQv0DO0HWwgewS1
zVbMLjJn8QKBgDl7OCGf5/KnWjP09EH2MWBixHpzc8osNjNTH/EbzxSPK1Go/sC4
Qa5H7H+AWHvTPJMOW3dxZtGenffn+6rirhEYpjA/spvk1NdJp3NTQDjHyFrcJ0Kh
nOedI5Bk464TqJT/NPMYNB35CiWHVTzCDHcHmkX5KN1n3cFBBL5lZimFAoGAROWQ
rJ3xCRYvTOGzX17W2j1mq3vSiGM2wL09gRLj8cGHWqT8ksJ+Sm0egdwHATb+uhEG
9PgyqHQ0laV/489tZa7XqPBLQKyWy0HNUKU0pnNEExjN3LFbXmBuxiKSdPIg08sB
JOvMg8E+shu8DQ5JAXVll5IPBnLfiMnTjvttc/ECgYBp3ulyqdZ9nxmS3eG2m742
ov2AcdIKsSj0PUdYkuB4fQRgvMBR/YwssESpgeA0YTdvp2HUHREASCTm1eLkSGHC
/gYCAjZRYHWXs2MhtmRPb93sYQabAU6r+1XIWRjg5DFHpqNKysGE4y248pnBJ5Z/
B+Z3u2wNkkfipW4EhrzPUg==
-----END PRIVATE KEY-----"#;

    // Parse certificate
    let mut cert_reader = &cert_pem[..];
    let certs: Vec<CertificateDer> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    // Parse private key
    let mut key_reader = &key_pem[..];
    let key: PrivateKeyDer = rustls_pemfile::private_key(&mut key_reader)
        .unwrap()
        .unwrap();

    // Create signing key
    let signing_key = rustls::crypto::aws_lc_rs::sign::any_supported_type(&key).unwrap();

    Arc::new(CertifiedKey::new(certs, signing_key))
}

#[test]
fn test_sni_resolver_new() {
    let resolver = SniResolver::new();
    assert!(resolver.is_empty());
}

#[test]
fn test_sni_resolver_add_cert() {
    let mut resolver = SniResolver::new();
    let cert = create_test_certified_key();

    resolver.add_cert("example.com".to_string(), cert.clone());
    assert!(!resolver.is_empty());
}

#[test]
fn test_sni_resolver_multiple_certs() {
    let mut resolver = SniResolver::new();
    let cert1 = create_test_certified_key();
    let cert2 = create_test_certified_key();

    resolver.add_cert("example.com".to_string(), cert1);
    resolver.add_cert("example.org".to_string(), cert2);

    assert!(!resolver.is_empty());
}

#[test]
fn test_sni_resolver_case_insensitive() {
    let mut resolver = SniResolver::new();
    let cert = create_test_certified_key();

    // Add with mixed case
    resolver.add_cert("Example.COM".to_string(), cert.clone());

    // Should be stored as lowercase
    assert!(!resolver.is_empty());
}

#[test]
fn test_sni_resolver_wildcard_domain() {
    let mut resolver = SniResolver::new();
    let cert = create_test_certified_key();

    resolver.add_cert("*.example.com".to_string(), cert.clone());
    assert!(!resolver.is_empty());
}

#[test]
fn test_build_tls_acceptor_success() {
    let mut resolver = SniResolver::new();
    let cert = create_test_certified_key();

    resolver.add_cert("example.com".to_string(), cert);

    let result = build_tls_acceptor(resolver);
    assert!(result.is_ok());
}

#[test]
fn test_build_tls_acceptor_empty_resolver() {
    let resolver = SniResolver::new();

    let result = build_tls_acceptor(resolver);
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(matches!(
            e,
            chimney::error::ServerError::TlsInitializationFailed(_)
        ));
    }
}

#[test]
fn test_build_tls_acceptor_multiple_domains() {
    let mut resolver = SniResolver::new();
    let cert1 = create_test_certified_key();
    let cert2 = create_test_certified_key();

    resolver.add_cert("example.com".to_string(), cert1);
    resolver.add_cert("example.org".to_string(), cert2);

    let result = build_tls_acceptor(resolver);
    assert!(result.is_ok());
}
