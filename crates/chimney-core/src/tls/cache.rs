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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_site_name_valid() {
        assert!(validate_site_name("example.com").is_ok());
        assert!(validate_site_name("my-site").is_ok());
        assert!(validate_site_name("site_123").is_ok());
        assert!(validate_site_name("example.co.uk").is_ok());
    }

    #[test]
    fn test_validate_site_name_path_traversal() {
        assert!(validate_site_name("../etc/passwd").is_err());
        assert!(validate_site_name("..").is_err());
        assert!(validate_site_name("site/../other").is_err());
        assert!(validate_site_name("site/subdir").is_err());
        assert!(validate_site_name("site\\subdir").is_err());
    }

    #[test]
    fn test_validate_site_name_empty() {
        assert!(validate_site_name("").is_err());
        assert!(validate_site_name("   ").is_err());
        assert!(validate_site_name("\t").is_err());
    }

    #[test]
    fn test_create_cert_directory_success() {
        let temp_dir = TempDir::new().unwrap();
        let cert_dir = temp_dir.path();

        let result = create_cert_directory("test-site", cert_dir);
        assert!(result.is_ok());

        let site_dir = result.unwrap();
        assert!(site_dir.exists());
        assert!(site_dir.is_dir());
        assert_eq!(site_dir.file_name().unwrap(), "test-site");
    }

    #[test]
    fn test_create_cert_directory_invalid_name() {
        let temp_dir = TempDir::new().unwrap();
        let cert_dir = temp_dir.path();

        let result = create_cert_directory("../evil", cert_dir);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServerError::TlsInitializationFailed(_)
        ));
    }

    #[test]
    fn test_save_and_load_certificate() {
        let temp_dir = TempDir::new().unwrap();
        let cert_dir = temp_dir.path();

        let cert_pem = b"-----BEGIN CERTIFICATE-----\ntest cert\n-----END CERTIFICATE-----";
        let key_pem = b"-----BEGIN PRIVATE KEY-----\ntest key\n-----END PRIVATE KEY-----";

        // Save certificate
        let save_result = save_certificate("test-site", cert_dir, cert_pem, key_pem);
        assert!(save_result.is_ok(), "Failed to save certificate: {:?}", save_result.err());

        // Verify files were created
        let site_dir = cert_dir.join("test-site");
        assert!(site_dir.join("cert.pem").exists());
        assert!(site_dir.join("key.pem").exists());

        // Load certificate back
        let load_result = load_cached_certificate("test-site", cert_dir);
        assert!(load_result.is_ok());

        let (loaded_cert, loaded_key) = load_result.unwrap().unwrap();
        assert_eq!(loaded_cert, cert_pem);
        assert_eq!(loaded_key, key_pem);
    }

    #[test]
    fn test_load_cached_certificate_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let cert_dir = temp_dir.path();

        let result = load_cached_certificate("nonexistent", cert_dir);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_save_certificate_invalid_site_name() {
        let temp_dir = TempDir::new().unwrap();
        let cert_dir = temp_dir.path();

        let cert_pem = b"cert";
        let key_pem = b"key";

        let result = save_certificate("../../etc/passwd", cert_dir, cert_pem, key_pem);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_private_key_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let cert_dir = temp_dir.path();

        let cert_pem = b"cert";
        let key_pem = b"key";

        save_certificate("test-site", cert_dir, cert_pem, key_pem).unwrap();

        let key_path = cert_dir.join("test-site").join("key.pem");
        let metadata = std::fs::metadata(&key_path).unwrap();
        let permissions = metadata.permissions();

        // Should be 0o600 (owner read/write only)
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }
}
