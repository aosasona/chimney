// Standalone certificate request functionality for manual ACME certificate issuance
//
// This module provides the ability to request TLS certificates programmatically
// without running the full server. Useful for:
// - Pre-provisioning certificates before server startup
// - Requesting certificates for sites added dynamically
// - CLI-based certificate management

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use log::{debug, error, info};
use tokio::net::TcpListener;
use tokio_rustls_acme::caches::DirCache;
use tokio_rustls_acme::AcmeConfig;

use crate::error::ServerError;

/// Default Let's Encrypt production directory URL
pub const LETS_ENCRYPT_PRODUCTION_URL: &str = "https://acme-v02.api.letsencrypt.org/directory";

/// Let's Encrypt staging directory URL (for testing)
pub const LETS_ENCRYPT_STAGING_URL: &str = "https://acme-staging-v02.api.letsencrypt.org/directory";

/// Options for requesting a TLS certificate via ACME
#[derive(Debug, Clone)]
pub struct CertRequestOptions {
    /// Domain names to request certificate for
    pub domains: Vec<String>,
    /// Email address for ACME account registration
    pub email: String,
    /// ACME directory URL (e.g., Let's Encrypt production or staging)
    pub directory_url: String,
    /// Directory to cache certificates
    pub cache_dir: PathBuf,
    /// Port to bind for ACME TLS-ALPN-01 challenge (default: 443)
    pub challenge_port: u16,
    /// Timeout for certificate issuance (default: 5 minutes)
    pub timeout: Duration,
    /// Host address to bind to (default: 0.0.0.0)
    pub bind_host: IpAddr,
}

impl Default for CertRequestOptions {
    fn default() -> Self {
        Self {
            domains: Vec::new(),
            email: String::new(),
            directory_url: LETS_ENCRYPT_PRODUCTION_URL.to_string(),
            cache_dir: PathBuf::from(".chimney/certs"),
            challenge_port: 443,
            timeout: Duration::from_secs(300), // 5 minutes
            bind_host: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        }
    }
}

/// Result of a successful certificate request
#[derive(Debug, Clone)]
pub struct CertRequestResult {
    /// Domain names the certificate was issued for
    pub domains: Vec<String>,
    /// Path to the certificate file
    pub cert_path: PathBuf,
    /// Path to the private key file
    pub key_path: PathBuf,
}

/// Request a TLS certificate for the specified domains via ACME
///
/// This function will:
/// 1. Bind to the specified port for ACME TLS-ALPN-01 validation
/// 2. Request a certificate from the ACME server
/// 3. Wait for the certificate to be issued (or timeout)
/// 4. Return the paths to the saved certificate files
///
/// # Requirements
/// - Port 443 (or specified `challenge_port`) must be available
/// - The domains must resolve to this server's IP address
/// - The ACME directory must be reachable
///
/// # Example
/// ```ignore
/// use chimney::tls::cert_request::{request_certificate, CertRequestOptions};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let options = CertRequestOptions {
///         domains: vec!["example.com".to_string()],
///         email: "admin@example.com".to_string(),
///         ..Default::default()
///     };
///
///     let result = request_certificate(options).await?;
///     println!("Certificate saved to: {:?}", result.cert_path);
///     Ok(())
/// }
/// ```
pub async fn request_certificate(options: CertRequestOptions) -> Result<CertRequestResult, ServerError> {
    // Validate options
    if options.domains.is_empty() {
        return Err(ServerError::TlsInitializationFailed(
            "At least one domain is required".to_string(),
        ));
    }

    if options.email.is_empty() {
        return Err(ServerError::TlsInitializationFailed(
            "ACME email is required".to_string(),
        ));
    }

    // Create site name from first domain (sanitized for filesystem)
    let site_name = options.domains[0]
        .replace('.', "_")
        .replace('*', "wildcard");

    // Validate site name and create cache directory
    let site_cache_dir = super::cache::create_cert_directory(&site_name, &options.cache_dir)?;

    info!(
        "Requesting certificate for domains: {:?}",
        options.domains
    );
    info!("Using ACME directory: {}", options.directory_url);
    info!("Certificates will be cached in: {}", site_cache_dir.display());

    // Install default crypto provider if not already installed
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Create ACME configuration
    // Note: DirCache::new takes ownership of the path
    let acme_config = AcmeConfig::new(options.domains.clone())
        .contact_push(format!("mailto:{}", options.email))
        .directory(&options.directory_url)
        .cache(DirCache::new(site_cache_dir.clone()));

    // Create ACME state
    let mut state = acme_config.state();
    let acceptor = state.acceptor();

    // Bind to challenge port
    let addr = SocketAddr::new(options.bind_host, options.challenge_port);
    let listener = TcpListener::bind(addr).await.map_err(|e| {
        ServerError::TlsInitializationFailed(format!(
            "Failed to bind to {} for ACME challenge: {}. \
             Ensure the port is available and you have sufficient permissions (port 443 typically requires root/admin).",
            addr, e
        ))
    })?;

    info!("Listening on {} for ACME TLS-ALPN-01 challenge", addr);

    // Track certificate issuance errors
    let cert_error = Arc::new(std::sync::Mutex::new(None::<String>));
    let cert_error_clone = cert_error.clone();
    let domains_for_log = options.domains.clone();

    // Spawn ACME event handler
    let event_handle = tokio::spawn(async move {
        loop {
            match state.next().await {
                Some(Ok(event)) => {
                    info!("ACME event for {:?}: {:?}", domains_for_log, event);
                    // The tokio-rustls-acme library fires events and writes certificates
                    // to the DirCache automatically. We just log events here and
                    // check for file existence in the main loop.
                }
                Some(Err(err)) => {
                    error!("ACME error: {:?}", err);
                    let mut error_guard = cert_error_clone.lock().unwrap();
                    *error_guard = Some(format!("{:?}", err));
                    break;
                }
                None => {
                    debug!("ACME state stream ended");
                    break;
                }
            }
        }
    });

    // Accept connections for ACME challenge
    let acceptor_clone = acceptor.clone();
    let accept_handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!("Accepted connection from {} for ACME challenge", peer_addr);

                    let acceptor_inner = acceptor_clone.clone();
                    tokio::spawn(async move {
                        match acceptor_inner.accept(stream).await {
                            Ok(None) => {
                                info!("Handled ACME TLS-ALPN-01 challenge from {}", peer_addr);
                            }
                            Ok(Some(_tls_stream)) => {
                                debug!("Regular TLS connection from {} (not an ACME challenge)", peer_addr);
                            }
                            Err(e) => {
                                error!("TLS accept error from {}: {}", peer_addr, e);
                            }
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    });

    // Wait for certificate with timeout
    let start = std::time::Instant::now();
    let cert_path = site_cache_dir.join("cert.pem");
    let key_path = site_cache_dir.join("key.pem");

    loop {
        // Check if we've exceeded timeout
        if start.elapsed() > options.timeout {
            // Cleanup
            accept_handle.abort();
            event_handle.abort();

            // Check if there was an error
            let error_guard = cert_error.lock().unwrap();
            if let Some(err) = error_guard.as_ref() {
                return Err(ServerError::AcmeCertificateIssuanceFailed(err.clone()));
            }

            return Err(ServerError::AcmeCertificateIssuanceFailed(
                "Timeout waiting for certificate issuance. Ensure domains resolve to this server.".to_string(),
            ));
        }

        // Check if certificate files exist (DirCache writes them automatically)
        if cert_path.exists() && key_path.exists() {
            // Verify the files are non-empty
            let cert_meta = std::fs::metadata(&cert_path).ok();
            let key_meta = std::fs::metadata(&key_path).ok();

            if let (Some(cert_m), Some(key_m)) = (cert_meta, key_meta) {
                if cert_m.len() > 0 && key_m.len() > 0 {
                    info!("Certificate files found and verified");
                    break;
                }
            }
        }

        // Check for errors
        {
            let error_guard = cert_error.lock().unwrap();
            if let Some(err) = error_guard.as_ref() {
                accept_handle.abort();
                event_handle.abort();
                return Err(ServerError::AcmeCertificateIssuanceFailed(err.clone()));
            }
        }

        // Wait a bit before checking again
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Cleanup
    accept_handle.abort();
    event_handle.abort();

    info!("Certificate issued successfully!");
    info!("Certificate: {}", cert_path.display());
    info!("Private key: {}", key_path.display());

    Ok(CertRequestResult {
        domains: options.domains,
        cert_path,
        key_path,
    })
}

/// Options builder for certificate requests.
///
/// Provides a fluent API for constructing `CertRequestOptions`.
///
/// # Example
/// ```
/// use chimney::tls::CertRequestOptionsBuilder;
///
/// let options = CertRequestOptionsBuilder::new()
///     .domain("example.com")
///     .domain("www.example.com")
///     .email("admin@example.com")
///     .staging()  // Use Let's Encrypt staging for testing
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct CertRequestOptionsBuilder {
    options: CertRequestOptions,
}

impl CertRequestOptionsBuilder {
    /// Create a new options builder with defaults.
    pub fn new() -> Self {
        Self {
            options: CertRequestOptions::default(),
        }
    }

    /// Add a domain to request the certificate for.
    ///
    /// This method is chainable and can be called multiple times.
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.options.domains.push(domain.into());
        self
    }

    /// Add multiple domains to request the certificate for.
    ///
    /// This method is chainable and can be called multiple times.
    pub fn domains<I, S>(mut self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for domain in domains {
            self.options.domains.push(domain.into());
        }
        self
    }

    /// Set the email address for ACME account registration.
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.options.email = email.into();
        self
    }

    /// Set the ACME directory URL.
    pub fn directory_url(mut self, url: impl Into<String>) -> Self {
        self.options.directory_url = url.into();
        self
    }

    /// Use Let's Encrypt staging environment (for testing).
    ///
    /// Staging certificates are not trusted by browsers but allow unlimited
    /// requests, making them ideal for testing.
    pub fn staging(mut self) -> Self {
        self.options.directory_url = LETS_ENCRYPT_STAGING_URL.to_string();
        self
    }

    /// Use Let's Encrypt production environment (default).
    pub fn production(mut self) -> Self {
        self.options.directory_url = LETS_ENCRYPT_PRODUCTION_URL.to_string();
        self
    }

    /// Set the certificate cache directory.
    pub fn cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.options.cache_dir = dir.into();
        self
    }

    /// Set the port to bind for ACME challenge.
    pub fn challenge_port(mut self, port: u16) -> Self {
        self.options.challenge_port = port;
        self
    }

    /// Set the timeout for certificate issuance.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.options.timeout = timeout;
        self
    }

    /// Set the host address to bind to.
    pub fn bind_host(mut self, host: IpAddr) -> Self {
        self.options.bind_host = host;
        self
    }

    /// Build the `CertRequestOptions`.
    pub fn build(self) -> CertRequestOptions {
        self.options
    }
}
