use std::io::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChimneyError {
    #[error("Unable to get current working directory and no target directory provided, reason: {source:?}")]
    FailedToGetWorkingDir { source: StdError },

    #[error("The target directory does not exist or is not a directory: {path:?}")]
    TargetDirNotExists { path: std::path::PathBuf },

    #[error("Failed to write config file, reason: {source:?}")]
    FailedToWriteConfig { source: StdError },

    #[error("Config file not found at the specified path `{0}`")]
    ConfigNotFound(String),

    #[error("Failed to read config file, reason: {source:?}")]
    FailedToReadConfig { source: StdError },
}

