use crate::log_warning;
use path_absolutize::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
    fs,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};

use crate::error::{ChimneyError, ChimneyError::*};

macro_rules! absolute_path_str {
    ($path:expr) => {
        match $path.absolutize() {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(e) => {
                log_warning!(format!("Failed to get absolute path: {}", e));
                $path.to_string_lossy().to_string()
            }
        }
    };
}

const CONFIG_TEMPLATE: &str = r#"host = "0.0.0.0"
port = 80
domain_names = [] # the domain names that the server will respond to
enable_logging = true # if true, the server will log all requests to the console
root_dir = "public" # the directory where the server will look for files to serve, relative to where this config file is located unless an absolute path is provided
fallback_document = "index.html" # whenever a request doesn't match a file, the server will serve this file


# [https]
# enable = false # if true, the server will use HTTPS
# auto_redirect = true # if true, the server will redirect all HTTP requests to HTTPS
# port = 443
# use_self_signed = false # if true, the server will use a self-signed certificate for SSL

# or you can use your own certificate for production use cases

# cert_file = "" # if `local_cert` is false, this should be the path to the SSL certificate
# key_file = "" # if `local_cert` is false, this should be the path to the SSL key

# [rewrites]
# the leading slash is required, if it is not present, the server will NOT recognize the path
# "/home" = { to = "/index.html" } # if a request is made to /home, the server will serve /index.html instead
# "/page-2" = "another_page.html" # this is relative to the root directory, so if the root directory is `public`, the server will serve `public/another_page.html` instead

# [headers]
# these headers will be added to every response
# "Cache-Control" = "no-cache, no-store, must-revalidate"
# "Pragma" = "no-cache"

# [redirects]
# the leading slash is required, if it is not present, the server will NOT recognize the path
# "/rick" = "https://www.youtube.com/watch?v=dQw4w9WgXcQ" # if a request is made to /rick, the server will redirect to the Rick Astley video
# "/google" = { to = "https://google.com", replay = true } # replay here means that the server will ask the browser to replay the request to the new location (HTTP status 308)
"#;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Rewrite {
    // This will take other config options in the future, that is why it is a struct
    Config {
        #[serde(default)]
        to: String,
    },

    Target(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Redirect {
    Config {
        #[serde(default)]
        to: String,

        #[serde(default)]
        replay: bool,
    },

    Target(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Https {
    #[serde(default = "Https::default_status")]
    enable: bool,

    #[serde(default)]
    auto_redirect: bool,

    #[serde(default = "Https::default_port")]
    pub port: u16,

    #[serde(default)]
    pub use_self_signed: bool,

    #[serde(default)]
    pub cert_file: Option<String>,

    #[serde(default)]
    pub key_file: Option<String>,
}

impl Https {
    fn default_port() -> u16 {
        443
    }

    fn default_status() -> bool {
        false
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "Config::default_host")]
    pub host: IpAddr,

    #[serde(default = "Config::default_port")]
    pub port: u16,

    #[serde(default)]
    pub domain_names: Vec<String>,

    #[serde(default = "Config::default_logging_flag")]
    pub enable_logging: bool,

    #[serde(default = "Config::default_root_dir")]
    pub root_dir: String,

    #[serde(default)]
    pub fallback_document: Option<String>,

    #[serde(default)]
    pub https: Option<Https>,

    #[serde(default)]
    pub headers: HashMap<String, String>,

    #[serde(default)]
    pub rewrites: HashMap<String, Rewrite>,

    #[serde(default)]
    pub redirects: HashMap<String, Redirect>,
}

impl Config {
    fn default_host() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
    }

    fn default_port() -> u16 {
        80
    }

    fn default_logging_flag() -> bool {
        true
    }

    fn default_root_dir() -> String {
        "public".to_string()
    }
}

pub fn init_at(path: &mut PathBuf) -> Result<String, ChimneyError> {
    if !path.is_dir() {
        return Err(TargetDirNotExists(absolute_path_str!(path)));
    }

    path.push("chimney.toml");
    if path.exists() {
        return Err(ConfigAlreadyExists(absolute_path_str!(path)));
    }

    fs::write(&path, CONFIG_TEMPLATE).map_err(FailedToWriteConfig)?;

    Ok(absolute_path_str!(path))
}

pub fn read_from_path(config_path: &mut PathBuf) -> Result<Config, ChimneyError> {
    // Try to find `chimney.toml` as a file or IN the target directory
    let has_toml_extension = config_path.extension().map_or(false, |ext| ext == "toml");

    if config_path.exists() && config_path.is_dir() {
        config_path.push("chimney.toml");
    }

    if !config_path.exists() {
        // We will pretend it is a directory if it doesn't have the `.toml` extension and return
        // that error
        if !has_toml_extension {
            return Err(TargetDirNotExists(absolute_path_str!(config_path)));
        }

        return Err(ConfigNotFound(absolute_path_str!(config_path)));
    }

    let raw_config = fs::read_to_string(config_path.clone()).map_err(FailedToReadConfig)?;

    let mut config: Config = toml::from_str(&raw_config).map_err(InvalidConfig)?;

    if config.root_dir.is_empty() {
        return Err(RootDirNotSet);
    }

    // Expand the root directory to an absolute path
    let current_dir = std::env::current_dir().map_err(FailedToGetWorkingDir)?;

    let parent_dir = if let Some(p) = config_path.parent() {
        p
    } else {
        current_dir.as_path()
    };

    config.root_dir = absolute_path_str!(parent_dir.join(config.root_dir.clone()));

    Ok(config)
}
