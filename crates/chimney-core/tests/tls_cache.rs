use chimney::error::ServerError;
use chimney::tls::cache::*;
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
