use std::path::Path;

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

    /// Creates a new certificate from path types.
    ///
    /// This is useful when working with `PathBuf` or `Path` values from filesystem operations.
    pub fn from_paths(cert: impl AsRef<Path>, key: impl AsRef<Path>) -> Self {
        Self {
            cert: cert.as_ref().to_string_lossy().into_owned(),
            key: key.as_ref().to_string_lossy().into_owned(),
            ca: None,
        }
    }

    /// Adds a CA bundle path to the certificate.
    pub fn with_ca(mut self, ca: impl Into<String>) -> Self {
        self.ca = Some(ca.into());
        self
    }

    /// Adds a CA bundle path from a path type.
    pub fn with_ca_path(mut self, ca: impl AsRef<Path>) -> Self {
        self.ca = Some(ca.as_ref().to_string_lossy().into_owned());
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
