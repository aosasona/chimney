use std::{io::Error as StdError, sync::Arc};
use thiserror::Error;
use tokio::sync::watch::error::SendError;

use crate::{config::Config, filesystem::FilesystemError};

#[derive(Error, Debug)]
pub enum ChimneyError {
    #[error("Error in `{field}`: {message}")]
    ConfigError { field: String, message: String },

    #[error("Failed to parse field `{field}`: {message}")]
    ParseError { message: String, field: String },

    #[error("{0}")]
    GenericError(String),

    #[error("{0}")]
    IOError(#[from] std::io::Error),

    #[error("Failed to parse Domain type: {0}")]
    DomainParseError(String),

    #[error("Domain `{domain}` already exists in the index")]
    DomainAlreadyExists { domain: String },
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("{0}")]
    FilesystemError(#[from] FilesystemError),

    #[error("Failed to parse raw address `{address}`: {message}")]
    InvalidRawSocketAddress { address: String, message: String },

    #[error("Invalid port range: {port}, must be between 1024 and 65535")]
    InvalidPortRange { port: u16 },

    #[error("Failed to bind to the specified address, reason: {0:?}")]
    FailedToBind(StdError),

    #[error("Failed to accept connection, reason: {0:?}")]
    FailedToAcceptConnection(StdError),

    #[error("Timeout waiting for connections to close")]
    TimeoutWaitingForConnections,

    #[error("No host detection method with valid target headers specified")]
    HostDetectionUnspecified,

    #[error("Failed to detect target host: {message}")]
    HostDetectionFailed { message: String },

    #[error(
        "No host header has been cached, cannot resolve host. This should not happen and is most likely a bug."
    )]
    MissingResolvedHostHeader,

    #[error("Invalid header value for `{header}`: `{value}`, reason: {message}")]
    InvalidHeaderValue {
        header: String,
        value: String,
        message: String,
    },

    #[error("No configured site found for domain `{host}`")]
    SiteNotFound { host: String },

    #[error("Failed to update configuration: {0}")]
    ConfigUpdateFailed(#[from] SendError<Arc<Config>>),

    // TLS-related errors
    #[error("TLS handshake failed: {0}")]
    TlsHandshakeFailed(String),

    #[error("TLS not configured but HTTPS connection attempted")]
    TlsNotConfigured,

    #[error("Failed to initialize TLS: {0}")]
    TlsInitializationFailed(String),

    #[error("ACME certificate issuance failed: {0}")]
    AcmeCertificateIssuanceFailed(String),

    #[error("Invalid certificate file at `{path}`: {message}")]
    InvalidCertificateFile { path: String, message: String },

    #[error("Invalid private key file at `{path}`: {message}")]
    InvalidPrivateKeyFile { path: String, message: String },

    #[error("Failed to create certificate directory at `{path}`: {message}")]
    CertificateDirectoryCreationFailed { path: String, message: String },
}
