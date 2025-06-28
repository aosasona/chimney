use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::ChimneyError;

/// Represents a domain name with an optional port number
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Hash)]
pub struct Domain {
    /// The domain name
    pub name: String,

    /// The port number (optional)
    pub port: Option<u16>,
}

impl Domain {
    /// Constructs a new `Domain` from a string representation
    pub fn new(name: String, port: Option<u16>) -> Self {
        Self { name, port }
    }
}

impl TryFrom<String> for Domain {
    type Error = ChimneyError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = if value.starts_with("http://") || value.starts_with("https://") {
            value
        } else {
            format!("http://{value}")
        };

        let url = Url::parse(&value).map_err(|e| {
            ChimneyError::DomainParseError(format!(
                "Failed to parse domain name '{value}': {e}"
            ))
        })?;
        let name = url
            .host_str()
            .ok_or_else(|| {
                ChimneyError::DomainParseError(format!(
                    "Invalid domain name '{value}': no host found"
                ))
            })?
            .to_string();

        let port = url.port();

        Ok(Domain { name, port })
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct DomainIndex {
    /// A precompiled index of domain names to site names for fast lookups
    inner: HashMap<Domain, String>,
}

impl DomainIndex {
    /// Inserts a domain into the index with the associated site name
    pub fn insert(&mut self, domain: Domain, site_name: String) -> Result<(), ChimneyError> {
        if self.inner.contains_key(&domain) {
            return Err(ChimneyError::DomainAlreadyExists {
                domain: domain.name.clone(),
            });
        }
        self.inner.insert(domain, site_name);

        Ok(())
    }

    /// Looks up a site name by domain
    pub fn get(&self, domain: &Domain) -> Option<&String> {
        self.inner.get(domain)
    }

    /// Checks if the index contains a domain
    pub fn contains(&self, domain: &Domain) -> bool {
        self.inner.contains_key(domain)
    }

    /// Removes all domains associated with a specific site name
    pub fn clear_for_site(&mut self, site_name: &str) {
        self.inner.retain(|_, v| v != site_name);
    }
}
