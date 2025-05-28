use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChimneyError {
    #[error("Error in `{field}`: {message}")]
    ConfigError { field: String, message: String },

    #[error("Failed to parse field `{field}`: {message}")]
    ParseError { message: String, field: String },

    #[error("{0}")]
    GenericError(String),
}
