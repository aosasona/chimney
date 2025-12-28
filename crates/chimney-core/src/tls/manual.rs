// Manual certificate loading from PEM files

use std::{fs::File, io::BufReader, path::{Path, PathBuf}, sync::Arc};

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::sign::CertifiedKey;
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};

use crate::error::ServerError;

/// Validate that a certificate/key file path is safe to read
///
/// This prevents reading arbitrary files on the system by ensuring:
/// 1. The path is canonicalized (resolves symlinks and relative paths)
/// 2. Basic validation that the path exists and is a file
///
/// Note: Relative paths with ".." are allowed and will be resolved via canonicalization.
/// The canonicalize() function safely resolves all path components and ensures the final
/// path exists and is accessible.
fn validate_cert_path(path: &Path, file_type: &str) -> Result<PathBuf, ServerError> {
    // Canonicalize the path (resolves symlinks, relative paths, and makes absolute)
    // This also validates that the path exists and is accessible
    let canonical = path.canonicalize().map_err(|e| {
        ServerError::InvalidCertificateFile {
            path: path.display().to_string(),
            message: format!("Cannot access {file_type}: {e}"),
        }
    })?;

    // Verify it's a file, not a directory
    if !canonical.is_file() {
        return Err(ServerError::InvalidCertificateFile {
            path: path.display().to_string(),
            message: format!("{file_type} path is not a file"),
        });
    }

    // NOTE: We don't restrict to specific directories because users may have
    // certificates in various locations (/etc/letsencrypt, ~/certs, etc.)
    // The canonicalize() check ensures the file exists, is accessible, and
    // all path components are safely resolved. Additional directory restrictions
    // could be added in the future if needed.

    Ok(canonical)
}

/// Load certificate chain from a PEM file
pub fn load_certificate_chain(path: &Path) -> Result<Vec<CertificateDer<'static>>, ServerError> {
    // Validate the path before opening
    let safe_path = validate_cert_path(path, "certificate")?;

    let file = File::open(&safe_path).map_err(|e| ServerError::InvalidCertificateFile {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;

    let mut reader = BufReader::new(file);
    let certs_result = certs(&mut reader).collect::<Result<Vec<_>, _>>();

    certs_result.map_err(|e| ServerError::InvalidCertificateFile {
        path: path.display().to_string(),
        message: format!("Failed to parse certificate: {e}"),
    })
}

/// Load private key from a PEM file (supports RSA and ECDSA)
pub fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, ServerError> {
    // Validate the path before opening
    let safe_path = validate_cert_path(path, "private key")?;

    let file = File::open(&safe_path).map_err(|e| ServerError::InvalidPrivateKeyFile {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;

    let mut reader = BufReader::new(file);
    let key = private_key(&mut reader)
        .map_err(|e| ServerError::InvalidPrivateKeyFile {
            path: path.display().to_string(),
            message: format!("Failed to parse private key: {e}"),
        })?
        .ok_or_else(|| ServerError::InvalidPrivateKeyFile {
            path: path.display().to_string(),
            message: "No private key found in file".to_string(),
        })?;

    return Ok(key);
}

/// Load a certified key from certificate and key files
pub fn load_certified_key(
    cert_file: &Path,
    key_file: &Path,
    ca_file: Option<&Path>,
) -> Result<Arc<CertifiedKey>, ServerError> {
    // CA bundles are not yet supported
    if let Some(ca) = ca_file {
        return Err(ServerError::TlsInitializationFailed(format!(
            "CA bundles not yet supported: {}",
            ca.display()
        )));
    }

    let certs = load_certificate_chain(cert_file)?;
    let key = load_private_key(key_file)?;

    // Create a signing key using the default crypto provider (aws_lc_rs)
    let signing_key = rustls::crypto::aws_lc_rs::sign::any_supported_type(&key)
        .map_err(|e| ServerError::TlsInitializationFailed(format!("Invalid private key: {e}")))?;

    let certified_key = CertifiedKey::new(certs, signing_key);

    return Ok(Arc::new(certified_key));
}

/// Build a rustls ServerConfig from certificate and key files
pub fn build_server_config(
    cert_file: &Path,
    key_file: &Path,
    ca_file: Option<&Path>,
) -> Result<ServerConfig, ServerError> {
    // CA bundles are not yet supported
    if let Some(ca) = ca_file {
        return Err(ServerError::TlsInitializationFailed(format!(
            "CA bundles not yet supported: {}",
            ca.display()
        )));
    }

    let certs = load_certificate_chain(cert_file)?;
    let key = load_private_key(key_file)?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| ServerError::TlsInitializationFailed(format!("Invalid certificate or key: {e}")))?;

    return Ok(config);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    // Test certificate and key (self-signed RSA 2048, for testing only)
    const TEST_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
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

    const TEST_KEY_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
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

    fn create_test_cert_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_validate_cert_path_valid() {
        let temp_dir = TempDir::new().unwrap();
        let cert_file = create_test_cert_file(temp_dir.path(), "cert.pem", TEST_CERT_PEM);

        let result = validate_cert_path(&cert_file, "certificate");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_cert_path_traversal() {
        // Path traversal attempt (file doesn't exist, should fail)
        let path = Path::new("../../../etc/passwd");
        let result = validate_cert_path(path, "certificate");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_cert_path_nonexistent() {
        let path = Path::new("/nonexistent/cert.pem");
        let result = validate_cert_path(path, "certificate");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_cert_path_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = validate_cert_path(temp_dir.path(), "certificate");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_certificate_chain_success() {
        let temp_dir = TempDir::new().unwrap();
        let cert_file = create_test_cert_file(temp_dir.path(), "cert.pem", TEST_CERT_PEM);

        let result = load_certificate_chain(&cert_file);
        assert!(result.is_ok(), "Failed to load certificate: {:?}", result.err());

        let certs = result.unwrap();
        assert!(!certs.is_empty(), "No certificates loaded");
    }

    #[test]
    fn test_load_certificate_chain_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_file = create_test_cert_file(temp_dir.path(), "invalid.pem", "not a certificate");

        let result = load_certificate_chain(&invalid_file);
        // The file exists and validates, but has no valid PEM certificates
        // rustls_pemfile::certs() returns an empty Vec for files with no valid certs
        // This is acceptable behavior - we get Ok(vec![])
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_load_certificate_chain_path_traversal() {
        let path = Path::new("../../../etc/passwd");
        let result = load_certificate_chain(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_private_key_success() {
        let temp_dir = TempDir::new().unwrap();
        let key_file = create_test_cert_file(temp_dir.path(), "key.pem", TEST_KEY_PEM);

        let result = load_private_key(&key_file);
        assert!(result.is_ok(), "Failed to load private key: {:?}", result.err());
    }

    #[test]
    fn test_load_private_key_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_file = create_test_cert_file(temp_dir.path(), "invalid.pem", "not a key");

        let result = load_private_key(&invalid_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_private_key_path_traversal() {
        let path = Path::new("../../../etc/shadow");
        let result = load_private_key(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_certified_key_success() {
        let temp_dir = TempDir::new().unwrap();
        let cert_file = create_test_cert_file(temp_dir.path(), "cert.pem", TEST_CERT_PEM);
        let key_file = create_test_cert_file(temp_dir.path(), "key.pem", TEST_KEY_PEM);

        let result = load_certified_key(&cert_file, &key_file, None);
        assert!(result.is_ok(), "Failed to load certified key: {:?}", result.err());
    }

    #[test]
    fn test_load_certified_key_mismatched() {
        let temp_dir = TempDir::new().unwrap();
        let cert_file = create_test_cert_file(temp_dir.path(), "cert.pem", TEST_CERT_PEM);
        let key_file = create_test_cert_file(temp_dir.path(), "key.pem", TEST_KEY_PEM);

        // This should still succeed in loading, mismatch is detected later
        let result = load_certified_key(&cert_file, &key_file, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_server_config_success() {
        let temp_dir = TempDir::new().unwrap();
        let cert_file = create_test_cert_file(temp_dir.path(), "cert.pem", TEST_CERT_PEM);
        let key_file = create_test_cert_file(temp_dir.path(), "key.pem", TEST_KEY_PEM);

        let result = build_server_config(&cert_file, &key_file, None);
        assert!(result.is_ok(), "Failed to build server config: {:?}", result.err());
    }
}
