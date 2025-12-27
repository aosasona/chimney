// Manual certificate loading from PEM files

use std::{fs::File, io::BufReader, path::Path, sync::Arc};

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::sign::CertifiedKey;
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};

use crate::error::ServerError;

/// Load certificate chain from a PEM file
pub fn load_certificate_chain(path: &Path) -> Result<Vec<CertificateDer<'static>>, ServerError> {
    let file = File::open(path).map_err(|e| ServerError::InvalidCertificateFile {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;

    let mut reader = BufReader::new(file);
    let certs_result = certs(&mut reader).collect::<Result<Vec<_>, _>>();

    certs_result.map_err(|e| ServerError::InvalidCertificateFile {
        path: path.display().to_string(),
        message: format!("Failed to parse certificate: {}", e),
    })
}

/// Load private key from a PEM file (supports RSA and ECDSA)
pub fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, ServerError> {
    let file = File::open(path).map_err(|e| ServerError::InvalidPrivateKeyFile {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;

    let mut reader = BufReader::new(file);
    let key = private_key(&mut reader)
        .map_err(|e| ServerError::InvalidPrivateKeyFile {
            path: path.display().to_string(),
            message: format!("Failed to parse private key: {}", e),
        })?
        .ok_or_else(|| ServerError::InvalidPrivateKeyFile {
            path: path.display().to_string(),
            message: "No private key found in file".to_string(),
        })?;

    Ok(key)
}

/// Load a certified key from certificate and key files
pub fn load_certified_key(
    cert_file: &Path,
    key_file: &Path,
    _ca_file: Option<&Path>,
) -> Result<Arc<CertifiedKey>, ServerError> {
    let certs = load_certificate_chain(cert_file)?;
    let key = load_private_key(key_file)?;

    // Create a signing key using the default crypto provider (aws_lc_rs)
    let signing_key = rustls::crypto::aws_lc_rs::sign::any_supported_type(&key)
        .map_err(|e| ServerError::TlsInitializationFailed(format!("Invalid private key: {}", e)))?;

    let certified_key = CertifiedKey::new(certs, signing_key);

    Ok(Arc::new(certified_key))
}

/// Build a rustls ServerConfig from certificate and key files
pub fn build_server_config(
    cert_file: &Path,
    key_file: &Path,
    _ca_file: Option<&Path>,
) -> Result<ServerConfig, ServerError> {
    let certs = load_certificate_chain(cert_file)?;
    let key = load_private_key(key_file)?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| ServerError::TlsInitializationFailed(format!("Invalid certificate or key: {}", e)))?;

    Ok(config)
}
