// TLS module for handling HTTPS connections, ACME, and certificate management

pub mod acceptor;
pub mod acme;
pub mod cache;
pub mod config;
pub mod manual;

use std::{path::Path, sync::Arc};

use log::{debug, info};
use rustls::crypto::CryptoProvider;
use tokio_rustls::TlsAcceptor;

use crate::{config::Config, error::ServerError};

use self::{
    acceptor::{build_tls_acceptor, SniResolver},
    acme::AcmeManager,
    config::{process_site_https_config, TlsMode},
};

/// Coordinates all TLS operations including certificate loading, ACME, and SNI
pub struct TlsManager {
    config: Arc<Config>,
    sni_resolver: SniResolver,
    acme_managers: Vec<AcmeManager>,
}

impl TlsManager {
    /// Create a new TLS manager from the configuration
    pub async fn new(config: Arc<Config>) -> Result<Self, ServerError> {
        debug!("Initializing TLS manager");

        // Install default crypto provider if not already installed
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let mut sni_resolver = SniResolver::new();
        let mut acme_managers = Vec::new();
        let cert_dir = config.cert_directory();

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
                        email,
                        directory_url,
                    } => {
                        // Initialize ACME manager for this site
                        info!(
                            "Initializing ACME for site '{}' with email: {}",
                            tls_config.site_name, email
                        );

                        let acme_manager = AcmeManager::new(
                            tls_config.site_name.clone(),
                            tls_config.domains.clone(),
                            email,
                            directory_url,
                            &cert_dir,
                        )
                        .await?;

                        acme_managers.push(acme_manager);
                    }
                }
            }
        }

        if sni_resolver.is_empty() && acme_managers.is_empty() {
            return Err(ServerError::TlsInitializationFailed(
                "No valid TLS certificates configured".to_string(),
            ));
        }

        Ok(Self {
            config,
            sni_resolver,
            acme_managers,
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

        // Note: ACME support is not yet fully implemented
        // For now, we only support manual certificates
        if !self.acme_managers.is_empty() {
            info!(
                "ACME configuration detected for {} site(s), but ACME is not yet fully implemented",
                self.acme_managers.len()
            );
            info!("Please use manual certificates for now");
        }

        // Build TLS acceptor with manual certificates only
        info!("Building TLS acceptor with manual certificates");
        let acceptor = build_tls_acceptor(self.sni_resolver.clone())?;
        Ok(Arc::new(acceptor))
    }
}
