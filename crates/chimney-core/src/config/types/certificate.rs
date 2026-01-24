use std::path::{Path, PathBuf};

/// Represents a TLS certificate with its associated key and optional CA bundle.
///
/// This struct is used to configure manual TLS certificates for sites.
///
/// # Example
/// ```
/// use chimney::config::Certificate;
///
/// // Basic certificate with cert and key
/// let cert = Certificate::new("./certs/cert.pem", "./certs/key.pem");
///
/// // Certificate with CA bundle
/// let cert_with_ca = Certificate::new("./certs/cert.pem", "./certs/key.pem")
///     .with_ca("./certs/ca.pem");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    /// Path to the certificate PEM file
    pub cert: String,
    /// Path to the private key PEM file
    pub key: String,
    /// Path to the CA bundle PEM file (optional)
    pub ca: Option<String>,
}

impl Certificate {
    /// Creates a new certificate with the given cert and key paths.
    pub fn new(cert: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            cert: cert.into(),
            key: key.into(),
            ca: None,
        }
    }

    /// Adds a CA bundle path to the certificate.
    pub fn with_ca(mut self, ca: impl Into<String>) -> Self {
        self.ca = Some(ca.into());
        self
    }

    /// Returns the certificate path as a `Path`.
    pub fn cert_path(&self) -> &Path {
        Path::new(&self.cert)
    }

    /// Returns the key path as a `Path`.
    pub fn key_path(&self) -> &Path {
        Path::new(&self.key)
    }

    /// Returns the CA path as an `Option<&Path>`.
    pub fn ca_path(&self) -> Option<&Path> {
        self.ca.as_ref().map(|s| Path::new(s.as_str()))
    }
}

/// A certificate with `PathBuf` paths, typically used for results from certificate operations.
///
/// This is similar to `Certificate` but uses owned `PathBuf` instead of `String`,
/// making it more suitable for representing file system paths in results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificatePaths {
    /// Path to the certificate PEM file
    pub cert: PathBuf,
    /// Path to the private key PEM file
    pub key: PathBuf,
    /// Path to the CA bundle PEM file (optional)
    pub ca: Option<PathBuf>,
}

impl CertificatePaths {
    /// Creates a new certificate paths struct.
    pub fn new(cert: impl Into<PathBuf>, key: impl Into<PathBuf>) -> Self {
        Self {
            cert: cert.into(),
            key: key.into(),
            ca: None,
        }
    }

    /// Adds a CA bundle path.
    pub fn with_ca(mut self, ca: impl Into<PathBuf>) -> Self {
        self.ca = Some(ca.into());
        self
    }
}

impl From<Certificate> for CertificatePaths {
    fn from(cert: Certificate) -> Self {
        Self {
            cert: PathBuf::from(cert.cert),
            key: PathBuf::from(cert.key),
            ca: cert.ca.map(PathBuf::from),
        }
    }
}

impl From<CertificatePaths> for Certificate {
    fn from(paths: CertificatePaths) -> Self {
        Self {
            cert: paths.cert.to_string_lossy().into_owned(),
            key: paths.key.to_string_lossy().into_owned(),
            ca: paths.ca.map(|p| p.to_string_lossy().into_owned()),
        }
    }
}
