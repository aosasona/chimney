// TLS module for handling HTTPS connections, ACME, and certificate management

pub mod acceptor;
pub mod acme;
pub mod cache;
pub mod config;
pub mod manual;

use std::{path::Path, sync::Arc};

use log::{debug, info};
use tokio_rustls::TlsAcceptor;

use crate::{config::Config, error::ServerError};

use self::{
    acceptor::{build_tls_acceptor, SniResolver},
    config::{process_site_https_config, TlsMode},
};

/// Coordinates all TLS operations including certificate loading, ACME, and SNI
pub struct TlsManager {
    config: Arc<Config>,
    sni_resolver: SniResolver,
}

impl TlsManager {
    /// Create a new TLS manager from the configuration
    pub async fn new(config: Arc<Config>) -> Result<Self, ServerError> {
        debug!("Initializing TLS manager");

        let mut sni_resolver = SniResolver::new();

        // Process each site's HTTPS configuration
        for site in config.sites.values() {
            if let Some(tls_config) = process_site_https_config(site)? {
                info!(
                    "Configuring TLS for site '{}' with domains: {:?}",
                    tls_config.site_name, tls_config.domains
                );

                match tls_config.mode {
                    TlsMode::Manual {
                        cert_file,
                        key_file,
                        ca_file,
                    } => {
                        // Load manual certificate
                        let cert_path = Path::new(&cert_file);
                        let key_path = Path::new(&key_file);
                        let ca_path = ca_file.as_deref().map(Path::new);

                        let certified_key =
                            manual::load_certified_key(cert_path, key_path, ca_path)?;

                        // Add certificate for each domain
                        for domain in &tls_config.domains {
                            debug!("Adding manual certificate for domain: {}", domain);
                            sni_resolver.add_cert(domain.clone(), certified_key.clone());
                        }
                    }
                    TlsMode::Acme {
                        email: _,
                        directory_url: _,
                    } => {
                        // ACME support will be implemented later
                        info!(
                            "ACME support not yet implemented for site '{}'",
                            tls_config.site_name
                        );
                        // TODO: Implement ACME certificate issuance
                    }
                }
            }
        }

        if sni_resolver.is_empty() {
            return Err(ServerError::TlsInitializationFailed(
                "No valid TLS certificates configured".to_string(),
            ));
        }

        Ok(Self {
            config,
            sni_resolver,
        })
    }

    /// Check if any site has HTTPS enabled
    pub fn is_tls_enabled(config: &Config) -> bool {
        config.sites.values().any(|site| {
            site.https_config
                .as_ref()
                .map(|https| https.enabled)
                .unwrap_or(false)
        })
    }

    /// Build a TLS acceptor with the configured certificates
    pub fn build_acceptor(&self) -> Result<Arc<TlsAcceptor>, ServerError> {
        debug!("Building TLS acceptor");
        let acceptor = build_tls_acceptor(self.sni_resolver.clone())?;
        Ok(Arc::new(acceptor))
    }
}
