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
