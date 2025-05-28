use toml::Table;

use crate::error::ChimneyError;

use super::{Config, Format, Site};

pub struct Toml<'a> {
    input: &'a str,
}

impl<'a> Toml<'a> {
    pub fn new(input: &'a str) -> Self {
        Toml { input }
    }
}
impl Toml<'_> {
    /// Parses the sites from the TOML table and adds them to the config
    fn parse_sites(self, config: &mut Config, sites: &Table) -> Result<(), ChimneyError> {
        for (key, value) in sites.iter() {
            let name = key.to_string();
            let site = Site::from_string(name, value.to_string());

            let site = site.map_err(|e| ChimneyError::ParseError {
                field: format!("sites.{}", key),
                message: format!("Failed to parse site `{}`: {}", key, e),
            })?;

            // If the site was parsed successfully, add it to the config
            config.sites.push((site.name.clone(), site));
        }

        Ok(())
    }
}

impl<'a> Format<'a> for Toml<'a> {
    fn set_input(mut self, input: &'a str) {
        self.input = input
    }

    fn parse(self) -> Result<super::Config, ChimneyError> {
        // Read the root configuration from the toml file
        let mut config: Config =
            toml::from_str(self.input).map_err(|e| ChimneyError::ParseError {
                field: "root".to_string(),
                message: format!("Failed to parse TOML configuration: {}", e),
            })?;

        // Read the sites configuration from the toml file if present
        let parsed = toml::from_str::<Table>(self.input).map_err(|e| ChimneyError::ParseError {
            field: "sites".to_string(),
            message: format!("Failed to parse sites configuration: {}", e),
        })?;

        if let Some(sites) = parsed.get("sites") {
            if let Some(sites) = sites.as_table() {
                self.parse_sites(&mut config, sites)?;
            }
        }

        Ok(config)
    }
}

