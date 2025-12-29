// SNI resolver and TLS acceptor creation

use std::{collections::HashMap, sync::Arc};

use rustls::{
    server::ResolvesServerCert,
    sign::CertifiedKey,
    ServerConfig,
};
use tokio_rustls::TlsAcceptor;

/// SNI resolver that maps domain names to certificates (manual certificates)
#[derive(Clone, Debug)]
pub struct SniResolver {
    certs: HashMap<String, Arc<CertifiedKey>>,
}

impl SniResolver {
    /// Create a new SNI resolver
    pub fn new() -> Self {
        Self {
            certs: HashMap::new(),
        }
    }

    /// Add a certificate for a domain
    pub fn add_cert(&mut self, domain: String, cert: Arc<CertifiedKey>) {
        self.certs.insert(domain.to_lowercase(), cert);
    }

    /// Check if resolver has any certificates
    pub fn is_empty(&self) -> bool {
        self.certs.is_empty()
    }
}

impl ResolvesServerCert for SniResolver {
    fn resolve(&self, client_hello: rustls::server::ClientHello) -> Option<Arc<CertifiedKey>> {
        let server_name = client_hello.server_name()?;
        let domain = server_name.to_lowercase();

        // Try exact match first
        if let Some(cert) = self.certs.get(&domain) {
            return Some(cert.clone());
        }

        // Try wildcard match (e.g., *.example.com matches foo.example.com)
        let parts: Vec<&str> = domain.split('.').collect();
        if parts.len() >= 2 {
            let wildcard = format!("*.{}", parts[1..].join("."));
            if let Some(cert) = self.certs.get(&wildcard) {
                return Some(cert.clone());
            }
        }

        None
    }
}

/// Build a TLS acceptor with SNI support (manual certificates only)
pub fn build_tls_acceptor(resolver: SniResolver) -> Result<TlsAcceptor, crate::error::ServerError> {
    if resolver.is_empty() {
        return Err(crate::error::ServerError::TlsInitializationFailed(
            "No certificates configured".to_string(),
        ));
    }

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));

    Ok(TlsAcceptor::from(Arc::new(config)))
}
