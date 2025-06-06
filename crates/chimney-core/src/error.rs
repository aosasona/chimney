use thiserror::Error;

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
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Failed to parse raw address `{address}`: {message}")]
    InvalidRawSocketAddress { address: String, message: String },

    #[error("Invalid port range: {port}, must be between 1024 and 65535")]
    InvalidPortRange { port: u16 },
}
