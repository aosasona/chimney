// SNI resolver and TLS acceptor creation

use std::{collections::HashMap, sync::Arc};

use rustls::{ServerConfig, server::ResolvesServerCert, sign::CertifiedKey};
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

    /// Resolve a certificate directly from an SNI server name.
    pub fn resolve_server_name(&self, server_name: &str) -> Option<Arc<CertifiedKey>> {
        let domain = server_name.to_lowercase();

        if let Some(cert) = self.certs.get(&domain) {
            return Some(cert.clone());
        }

        let parts: Vec<&str> = domain.split('.').collect();
        if parts.len() >= 2 {
            self.certs
                .get(&format!("*.{}", parts[1..].join(".")))
                .cloned()
        } else {
            None
        }
    }
}

impl ResolvesServerCert for SniResolver {
    fn resolve(&self, client_hello: rustls::server::ClientHello) -> Option<Arc<CertifiedKey>> {
        self.resolve_server_name(client_hello.server_name()?)
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
