#[cfg(feature = "toml")]
pub mod toml;

mod types;

pub use types::*;

use crate::error::ChimneyError;

pub trait Format<'a> {
    /// Set the input document
    fn set_input(self, input: &'a str);

    /// Parse the provided document and return a fully parsed config
    fn parse(self) -> Result<Config, ChimneyError>;
}
