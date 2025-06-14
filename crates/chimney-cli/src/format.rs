use chimney::config::{Format, toml};
use clap::ValueEnum;
use serde::Serialize;

#[derive(ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FormatType {
    /// TOML format
    #[default]
    Toml,
}

impl FormatType {
    /// Get the asspciated format for the type
    pub fn format<'a>(&self, input: &'a str) -> Box<dyn Format<'a> + 'a> {
        match self {
            FormatType::Toml => Box::new(toml::Toml::from(input)),
        }
    }
}
