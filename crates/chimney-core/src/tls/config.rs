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
    /// Automatic certificate issuance via ACME (uses global AcmeConfig)
    Acme,
    /// Manual certificate files
    Manual {
        cert_file: String,
        key_file: String,
        ca_file: Option<String>,
    },
}

/// Process HTTPS configuration for a site.
///
/// When global HTTPS is enabled, all sites get HTTPS:
/// - Sites with `cert_file` + `key_file` use manual certificates
/// - All other sites use ACME automatic certificate issuance
pub fn process_site_https_config(site: &Site) -> Result<TlsConfig, ServerError> {
    // Check for per-site overrides
    let mode = if let Some(https_config) = &site.https_config {
        // Validate per-site config
        https_config
            .validate(&site.name)
            .map_err(|e| ServerError::TlsInitializationFailed(format!("Invalid HTTPS config: {e}")))?;

        if https_config.is_manual() {
            TlsMode::Manual {
                cert_file: https_config.cert_file.clone().expect("validated"),
                key_file: https_config.key_file.clone().expect("validated"),
                ca_file: https_config.ca_file.clone(),
            }
        } else {
            TlsMode::Acme
        }
    } else {
        // No per-site config = use ACME
        TlsMode::Acme
    };

    Ok(TlsConfig {
        site_name: site.name.clone(),
        domains: site.domain_names.clone(),
        mode,
    })
}
