use crate::error::ChimneyError;

use super::Config;

pub trait Format<'a> {
    /// Set the input document
    fn set_input(&mut self, input: &'a str);

    /// Parse the provided document and return a fully parsed config
    fn parse(&self) -> Result<Config, ChimneyError>;

    /// Create a new instance of the format from the input string
    fn from_str(input: &'a str) -> Self
    where
        Self: Sized;

    /// Convert the format to a string representation
    fn to_format_string(&self, config: &Config) -> Result<String, ChimneyError>;

    /// Get the file extension for the format
    fn extension(&self) -> &'static str;
}
