// ACME client integration for automatic certificate issuance
//
// NOTE: Full ACME implementation with tokio-rustls-acme 0.8 is a work in progress.
// The API has changed significantly and requires additional integration work.
// For now, manual certificates are fully supported and recommended for production use.
//
// To use manual certificates, configure your site with:
// ```toml
// [https_config]
// enabled = true
// auto_issue = false
// cert_file = "/path/to/cert.pem"
// key_file = "/path/to/key.pem"
// ```

use std::path::Path;

use log::info;

use crate::error::ServerError;

/// ACME manager for a site (stub for future implementation)
pub struct AcmeManager {
    site_name: String,
    domains: Vec<String>,
    _email: String,
    _directory_url: String,
    _cache_dir: std::path::PathBuf,
}

impl AcmeManager {
    /// Create a new ACME manager (stub)
    pub async fn new(
        site_name: String,
        domains: Vec<String>,
        email: String,
        directory_url: String,
        cache_dir: &Path,
    ) -> Result<Self, ServerError> {
        info!(
            "ACME requested for site '{}' with domains: {:?}, but full ACME support is not yet implemented",
            site_name, domains
        );
        info!(
            "Please use manual certificates for now. See documentation for configuration."
        );

        Ok(Self {
            site_name,
            domains,
            _email: email,
            _directory_url: directory_url,
            _cache_dir: cache_dir.to_path_buf(),
        })
    }

}
