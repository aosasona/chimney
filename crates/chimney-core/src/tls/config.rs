// TLS configuration processing and validation

use crate::{config::Site, error::ServerError};

/// Processed TLS configuration for a site
#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub site_name: String,
    pub domains: Vec<String>,
    pub mode: TlsMode,
}

/// TLS mode for a site
#[derive(Debug, Clone)]
pub enum TlsMode {
    /// Automatic certificate issuance via ACME
    Acme {
        email: String,
        directory_url: String,
    },
    /// Manual certificate files
    Manual {
        cert_file: String,
        key_file: String,
        ca_file: Option<String>,
    },
}

/// Process and validate HTTPS configuration for a site
pub fn process_site_https_config(site: &Site) -> Result<Option<TlsConfig>, ServerError> {
    let https_config = match &site.https_config {
        Some(config) if config.enabled => config,
        _ => return Ok(None),
    };

    // Validate the configuration
    https_config.validate(&site.name).map_err(|e| {
        ServerError::TlsInitializationFailed(format!("Invalid HTTPS config: {}", e))
    })?;

    // Determine the mode
    let mode = if https_config.auto_issue {
        TlsMode::Acme {
            email: https_config
                .acme_email
                .clone()
                .ok_or_else(|| ServerError::AcmeEmailRequired {
                    site: site.name.clone(),
                })?,
            directory_url: https_config.acme_directory.clone(),
        }
    } else {
        TlsMode::Manual {
            cert_file: https_config
                .cert_file
                .clone()
                .ok_or_else(|| ServerError::InvalidCertificateFile {
                    path: String::new(),
                    message: "cert_file is required for manual certificates".to_string(),
                })?,
            key_file: https_config
                .key_file
                .clone()
                .ok_or_else(|| ServerError::InvalidPrivateKeyFile {
                    path: String::new(),
                    message: "key_file is required for manual certificates".to_string(),
                })?,
            ca_file: https_config.ca_file.clone(),
        }
    };

    Ok(Some(TlsConfig {
        site_name: site.name.clone(),
        domains: site.domain_names.clone(),
        mode,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Https, Site};
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
    fn test_process_site_https_config_disabled() {
        let site = create_test_site("test", vec!["example.com".to_string()], None);
        let result = process_site_https_config(&site);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_process_site_https_config_acme() {
        let https = Https {
            enabled: true,
            auto_issue: true,
            auto_redirect: true,
            cert_file: None,
            key_file: None,
            ca_file: None,
            acme_email: Some("admin@example.com".to_string()),
            acme_directory: "https://acme-staging.api.letsencrypt.org/directory".to_string(),
        };

        let site = create_test_site("test", vec!["example.com".to_string()], Some(https));
        let result = process_site_https_config(&site);

        assert!(result.is_ok());
        let config = result.unwrap().unwrap();

        assert_eq!(config.site_name, "test");
        assert_eq!(config.domains, vec!["example.com"]);
        assert!(matches!(config.mode, TlsMode::Acme { .. }));
    }

    #[test]
    fn test_process_site_https_config_manual() {
        let https = Https {
            enabled: true,
            auto_issue: false,
            auto_redirect: true,
            cert_file: Some("/path/to/cert.pem".to_string()),
            key_file: Some("/path/to/key.pem".to_string()),
            ca_file: Some("/path/to/ca.pem".to_string()),
            acme_email: None,
            acme_directory: Https::default_acme_directory(),
        };

        let site = create_test_site("test", vec!["example.com".to_string()], Some(https));
        let result = process_site_https_config(&site);

        assert!(result.is_ok());
        let config = result.unwrap().unwrap();

        assert_eq!(config.site_name, "test");
        assert!(matches!(config.mode, TlsMode::Manual { .. }));

        if let TlsMode::Manual { cert_file, key_file, ca_file } = config.mode {
            assert_eq!(cert_file, "/path/to/cert.pem");
            assert_eq!(key_file, "/path/to/key.pem");
            assert_eq!(ca_file, Some("/path/to/ca.pem".to_string()));
        }
    }

    #[test]
    fn test_process_site_https_config_acme_missing_email() {
        let https = Https {
            enabled: true,
            auto_issue: true,
            auto_redirect: true,
            cert_file: None,
            key_file: None,
            ca_file: None,
            acme_email: None,  // Missing required email
            acme_directory: Https::default_acme_directory(),
        };

        let site = create_test_site("test", vec!["example.com".to_string()], Some(https));
        let result = process_site_https_config(&site);

        assert!(result.is_err());
    }

    #[test]
    fn test_process_site_https_config_manual_missing_key() {
        let https = Https {
            enabled: true,
            auto_issue: false,
            auto_redirect: true,
            cert_file: Some("/path/to/cert.pem".to_string()),
            key_file: None,  // Missing required key file
            ca_file: None,
            acme_email: None,
            acme_directory: Https::default_acme_directory(),
        };

        let site = create_test_site("test", vec!["example.com".to_string()], Some(https));
        let result = process_site_https_config(&site);

        assert!(result.is_err());
    }

    #[test]
    fn test_process_site_https_config_conflicting() {
        let https = Https {
            enabled: true,
            auto_issue: true,
            auto_redirect: true,
            acme_email: Some("admin@example.com".to_string()),
            cert_file: Some("/path/to/cert.pem".to_string()),  // Conflicting
            key_file: Some("/path/to/key.pem".to_string()),
            ca_file: None,
            acme_directory: Https::default_acme_directory(),
        };

        let site = create_test_site("test", vec!["example.com".to_string()], Some(https));
        let result = process_site_https_config(&site);

        // Should fail validation due to conflicting auto_issue and manual certs
        assert!(result.is_err());
    }

    #[test]
    fn test_process_site_https_config_multiple_domains() {
        let https = Https {
            enabled: true,
            auto_issue: true,
            auto_redirect: true,
            cert_file: None,
            key_file: None,
            ca_file: None,
            acme_email: Some("admin@example.com".to_string()),
            acme_directory: Https::default_acme_directory(),
        };

        let domains = vec![
            "example.com".to_string(),
            "www.example.com".to_string(),
            "api.example.com".to_string(),
        ];

        let site = create_test_site("test", domains.clone(), Some(https));
        let result = process_site_https_config(&site);

        assert!(result.is_ok());
        let config = result.unwrap().unwrap();
        assert_eq!(config.domains, domains);
    }
}
