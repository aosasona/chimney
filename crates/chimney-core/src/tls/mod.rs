// TLS module for handling HTTPS connections, ACME, and certificate management
//
//! TLS and ACME certificate management for Chimney.
//!
//! This module provides automatic TLS certificate issuance via ACME (Let's Encrypt)
//! and manual certificate loading. It integrates seamlessly whether you're using
//! Chimney as a CLI application or as a library.
//!
//! # Library Usage
//!
//! When using Chimney as a library, HTTPS and ACME work automatically based on your configuration:
//!
//! ```ignore
//! use std::sync::Arc;
//! use chimney::{Config, Server, filesystem::LocalFilesystem};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create configuration with ACME
//!     let mut config = Config::default();
//!     config.sites_directory = "./sites".to_string();
//!
//!     // Optional: Set where config file would be (for cert caching location)
//!     config.config_file_path = Some(std::path::PathBuf::from("./config.toml"));
//!
//!     // Sites with HTTPS configured will automatically get ACME certificates
//!     // (configured in individual site chimney.toml files)
//!
//!     let filesystem = Arc::new(LocalFilesystem::new());
//!     let server = Server::new_with_tls(filesystem, Arc::new(config)).await?;
//!
//!     server.run().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Certificate Storage
//!
//! Certificates are cached in `.chimney/certs/` relative to:
//! - The directory containing the config file (if `config_file_path` is set)
//! - The `sites_directory` (if `config_file_path` is not set)
//!
//! This ensures certificates persist across restarts and are automatically renewed.

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
    acme::AcmeManager,
    config::{process_site_https_config, TlsMode},
};

/// Coordinates all TLS operations including certificate loading, ACME, and SNI
pub struct TlsManager {
    _config: Arc<Config>,
    sni_resolver: SniResolver,
    acme_manager: Option<AcmeManager>,
}

impl TlsManager {
    /// Create a new TLS manager from the configuration
    pub async fn new(config: Arc<Config>) -> Result<Self, ServerError> {
        debug!("Initializing TLS manager");

        // Install default crypto provider if not already installed
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let mut sni_resolver = SniResolver::new();
        let mut acme_domains = Vec::new();
        let mut acme_email = None;
        let mut acme_directory = None;
        let cert_dir = config.cert_directory();

        // First pass: collect manual certs and ACME domains
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
                        // Collect ACME domains
                        info!(
                            "Collecting ACME domains for site '{}': {:?}",
                            tls_config.site_name, tls_config.domains
                        );
                        acme_domains.extend(tls_config.domains.clone());

                        // Use the first ACME configuration's email and directory
                        // (all sites should use the same ACME settings)
                        if acme_email.is_none() {
                            acme_email = Some(email);
                            acme_directory = Some(directory_url);
                        }
                    }
                }
            }
        }

        // Create single ACME manager for all ACME domains
        let acme_manager = if !acme_domains.is_empty() {
            let email = acme_email.ok_or_else(|| {
                ServerError::TlsInitializationFailed(
                    "ACME email not configured".to_string()
                )
            })?;
            let directory = acme_directory.ok_or_else(|| {
                ServerError::TlsInitializationFailed(
                    "ACME directory not configured".to_string()
                )
            })?;

            info!(
                "Creating ACME manager for {} domain(s): {:?}",
                acme_domains.len(),
                acme_domains
            );

            Some(
                AcmeManager::new(
                    "acme-manager".to_string(),
                    acme_domains,
                    email,
                    directory,
                    &cert_dir,
                )
                .await?,
            )
        } else {
            None
        };

        if sni_resolver.is_empty() && acme_manager.is_none() {
            return Err(ServerError::TlsInitializationFailed(
                "No valid TLS certificates configured".to_string(),
            ));
        }

        Ok(Self {
            _config: config,
            sni_resolver,
            acme_manager,
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

    /// Check if ACME is enabled
    pub fn has_acme(&self) -> bool {
        self.acme_manager.is_some()
    }

    /// Check if manual certificates SNI resolver is empty
    pub fn is_manual_empty(&self) -> bool {
        self.sni_resolver.is_empty()
    }

    /// Get the ACME acceptor if ACME is enabled
    pub fn acme_acceptor(&self) -> Option<&tokio_rustls_acme::AcmeAcceptor> {
        self.acme_manager.as_ref().map(|m| m.acceptor())
    }

    /// Get the ACME resolver if ACME is enabled
    pub fn acme_resolver(&self) -> Option<Arc<dyn rustls::server::ResolvesServerCert>> {
        self.acme_manager.as_ref().map(|m| m.resolver())
    }

    /// Build a TLS acceptor with manual certificates only
    ///
    /// Note: This is only used when ACME is not enabled.
    /// When ACME is enabled, use acme_acceptor() instead.
    pub fn build_acceptor(&self) -> Result<Arc<TlsAcceptor>, ServerError> {
        debug!("Building TLS acceptor for manual certificates");

        if self.sni_resolver.is_empty() {
            return Err(ServerError::TlsInitializationFailed(
                "No manual certificates configured".to_string(),
            ));
        }

        let acceptor = build_tls_acceptor(self.sni_resolver.clone())?;
        Ok(Arc::new(acceptor))
    }
}
