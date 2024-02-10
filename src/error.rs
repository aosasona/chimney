use std::io::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChimneyError {
    #[error("Config file already exists at the specified path `{0}`")]
    ConfigAlreadyExists(String),

    #[error("Config file not found at the specified path `{0}`")]
    ConfigNotFound(String),

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
}

