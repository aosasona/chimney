// Certificate persistence and cache management

use std::{fs, path::{Path, PathBuf}};

use crate::error::ServerError;

/// Validate that a site name doesn't contain path traversal attempts
fn validate_site_name(site_name: &str) -> Result<(), ServerError> {
    // Check for path traversal attempts
    if site_name.contains("..") || site_name.contains('/') || site_name.contains('\\') {
        return Err(ServerError::TlsInitializationFailed(
            "Invalid site name: contains path traversal characters".to_string(),
        ));
    }

    // Check for empty or whitespace-only names
    if site_name.trim().is_empty() {
        return Err(ServerError::TlsInitializationFailed(
            "Invalid site name: empty or whitespace-only".to_string(),
        ));
    }

    Ok(())
}

/// Create the certificate directory for a site
pub fn create_cert_directory(site_name: &str, cert_dir: &Path) -> Result<PathBuf, ServerError> {
    // Validate site name to prevent path traversal
    validate_site_name(site_name)?;

    let site_cert_dir = cert_dir.join(site_name);

    fs::create_dir_all(&site_cert_dir).map_err(|e| {
        ServerError::CertificateDirectoryCreationFailed {
            path: format!(".chimney/certs/{}", site_name),  // Don't leak full path
            message: e.to_string(),
        }
    })?;

    Ok(site_cert_dir)
}

/// Save certificate and key to disk (atomic write)
pub fn save_certificate(
    site_name: &str,
    cert_dir: &Path,
    cert_pem: &[u8],
    key_pem: &[u8],
) -> Result<(), ServerError> {
    // create_cert_directory already validates site_name
    let site_cert_dir = create_cert_directory(site_name, cert_dir)?;

    let cert_path = site_cert_dir.join("cert.pem");
    let key_path = site_cert_dir.join("key.pem");

    // Write certificate
    let temp_cert = site_cert_dir.join(".cert.pem.tmp");
    fs::write(&temp_cert, cert_pem).map_err(|e| ServerError::TlsInitializationFailed(format!(
        "Failed to write certificate: {}",
        e
    )))?;
    fs::rename(&temp_cert, &cert_path).map_err(|e| ServerError::TlsInitializationFailed(format!(
        "Failed to move certificate: {}",
        e
    )))?;

    // Write private key with restricted permissions
    let temp_key = site_cert_dir.join(".key.pem.tmp");
    fs::write(&temp_key, key_pem).map_err(|e| ServerError::TlsInitializationFailed(format!(
        "Failed to write private key: {}",
        e
    )))?;

    // Set restrictive permissions on private key
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_key)
            .map_err(|e| ServerError::TlsInitializationFailed(format!(
                "Failed to get key file permissions: {}",
                e
            )))?
            .permissions();
        perms.set_mode(0o600);  // Owner read/write only
        fs::set_permissions(&temp_key, perms).map_err(|e| {
            ServerError::TlsInitializationFailed(format!("Failed to set key file permissions: {}", e))
        })?;
    }

    // NOTE: On Windows, file permissions are managed via ACLs and require
    // additional dependencies (winapi). Consider using proper ACLs for production.
    // For now, Windows users should ensure proper NTFS permissions manually.

    fs::rename(&temp_key, &key_path).map_err(|e| ServerError::TlsInitializationFailed(format!(
        "Failed to move private key: {}",
        e
    )))?;

    Ok(())
}

/// Load cached certificate and key from disk
pub fn load_cached_certificate(
    site_name: &str,
    cert_dir: &Path,
) -> Result<Option<(Vec<u8>, Vec<u8>)>, ServerError> {
    // Validate site name to prevent path traversal
    validate_site_name(site_name)?;

    let site_cert_dir = cert_dir.join(site_name);
    let cert_path = site_cert_dir.join("cert.pem");
    let key_path = site_cert_dir.join("key.pem");

    if !cert_path.exists() || !key_path.exists() {
        return Ok(None);
    }

    let cert_pem = fs::read(&cert_path).map_err(|e| ServerError::InvalidCertificateFile {
        path: format!(".chimney/certs/{}/cert.pem", site_name),  // Don't leak full path
        message: e.to_string(),
    })?;

    let key_pem = fs::read(&key_path).map_err(|e| ServerError::InvalidPrivateKeyFile {
        path: format!(".chimney/certs/{}/key.pem", site_name),  // Don't leak full path
        message: e.to_string(),
    })?;

    Ok(Some((cert_pem, key_pem)))
}
