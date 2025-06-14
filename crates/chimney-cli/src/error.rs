use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum CliError {
    #[error("Failed to read the configuration file: {0}")]
    Read(#[from] std::io::Error),

    #[error("{0}")]
    Chimney(#[from] chimney::error::ChimneyError),

    #[error("{0}")]
    Generic(String),
}
