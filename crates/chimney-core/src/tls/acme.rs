// ACME client integration for automatic certificate issuance using tokio-rustls-acme
//
// This module provides automatic TLS certificate management using Let's Encrypt via the ACME protocol.
// It uses TLS-ALPN-01 validation, which serves ACME challenges on the same port as regular TLS traffic.

use std::path::Path;
use std::sync::Arc;

use futures_util::StreamExt;
use log::{error, info};
use rustls::server::ResolvesServerCert;
use tokio_rustls_acme::caches::DirCache;
use tokio_rustls_acme::{AcmeAcceptor, AcmeConfig};

use crate::error::ServerError;

/// ACME manager for a site
///
/// Manages automatic certificate issuance and renewal using the ACME protocol.
/// Uses TLS-ALPN-01 validation which serves challenges on the HTTPS port.
pub struct AcmeManager {
    site_name: String,
    domains: Vec<String>,
    acceptor: AcmeAcceptor,
    resolver: Arc<dyn ResolvesServerCert>,
}

impl AcmeManager {
    /// Create a new ACME manager
    ///
    /// # Arguments
    /// * `site_name` - Name of the site for logging
    /// * `domains` - List of domain names to request certificates for
    /// * `email` - Contact email address (will be prefixed with "mailto:")
    /// * `directory_url` - ACME directory URL (e.g., Let's Encrypt production or staging)
    /// * `cache_dir` - Directory to cache certificates and account information
    pub async fn new(
        site_name: String,
        domains: Vec<String>,
        email: String,
        directory_url: String,
        cache_dir: &Path,
    ) -> Result<Self, ServerError> {
        info!(
            "Initializing ACME for site '{}' with domains: {:?}",
            site_name, domains
        );
        info!("Using ACME directory: {}", directory_url);

        // Validate site name to prevent path traversal
        if site_name.contains("..") || site_name.contains('/') || site_name.contains('\\') {
            return Err(ServerError::TlsInitializationFailed(
                format!("Invalid site name '{}': contains path traversal characters", site_name),
            ));
        }

        // Create cache directory for this site
        let site_cache_dir = cache_dir.join(&site_name);
        if !site_cache_dir.exists() {
            std::fs::create_dir_all(&site_cache_dir).map_err(|e| {
                ServerError::CertificateDirectoryCreationFailed {
                    path: site_cache_dir.display().to_string(),
                    message: e.to_string(),
                }
            })?;
        }

        // Create ACME configuration
        let config = AcmeConfig::new(domains.clone())
            .contact_push(format!("mailto:{}", email))
            .directory(directory_url)
            .cache(DirCache::new(site_cache_dir));

        // Create ACME state
        let mut state = config.state();
        let acceptor = state.acceptor();
        let resolver = state.resolver();

        // Spawn background task to handle ACME events (certificate issuance/renewal)
        let site_name_clone = site_name.clone();
        tokio::spawn(async move {
            loop {
                match state.next().await {
                    Some(Ok(event)) => {
                        info!("ACME event for site '{}': {:?}", site_name_clone, event);
                    }
                    Some(Err(err)) => {
                        error!("ACME error for site '{}': {:?}", site_name_clone, err);
                    }
                    None => {
                        info!("ACME state stream ended for site '{}'", site_name_clone);
                        break;
                    }
                }
            }
        });

        info!(
            "ACME manager initialized for site '{}' with {} domain(s)",
            site_name,
            domains.len()
        );

        Ok(Self {
            site_name,
            domains,
            acceptor,
            resolver,
        })
    }

    /// Get the ACME acceptor for handling TLS connections
    pub fn acceptor(&self) -> &AcmeAcceptor {
        &self.acceptor
    }

    /// Get the ACME resolver for certificate resolution
    pub fn resolver(&self) -> Arc<dyn ResolvesServerCert> {
        self.resolver.clone()
    }

    /// Get the domains managed by this ACME manager
    pub fn domains(&self) -> &[String] {
        &self.domains
    }

    /// Get the site name
    pub fn site_name(&self) -> &str {
        &self.site_name
    }
}
