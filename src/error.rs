use std::{io::Error as StdError, net::AddrParseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChimneyError {
    #[error("Config file already exists at the specified path `{0}`")]
    ConfigAlreadyExists(String),

    #[error("Config file not found at the specified path `{0}`")]
    ConfigNotFound(String),

    #[error("Failed to parse address ({0}), reason: {1:?}")]
    FailedToParseAddress(String, AddrParseError),

    #[error(
        "Unable to get current working directory and no target directory provided, reason: {0:?}"
    )]
    FailedToGetWorkingDir(StdError),

    #[error("Failed to write config file, reason: {0:?}")]
    FailedToWriteConfig(StdError),

    #[error("Failed to read config file, reason: {0:?}")]
    FailedToReadConfig(StdError),

    #[error("Invalid config file, reason: {0:?}")]
    InvalidConfig(toml::de::Error),

    #[error("The target directory does not exist or is not a directory: {0}")]
    TargetDirNotExists(String),

    #[error("Failed to bind to the specified address, reason: {0:?}")]
    FailedToBind(StdError),

    #[error("Failed to accept connection, reason: {0:?}")]
    FailedToAcceptConnection(StdError),

    #[error("Root directory not set in the config file, this is required for the server to run")]
    RootDirNotSet,

    #[error("Failed to open file, reason: {0:?}")]
    UnableToOpenFile(StdError),
}
