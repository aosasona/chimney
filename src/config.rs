use crate::log_warning;
use path_absolutize::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
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
mode = "single"
root = "public" # the directory where the server will look for files to serve, relative to where this config file is located unless an absolute path is provided
# or you can do this:
# root = {
#    path = "public",
#    ignore_matches = [] # this only applies if you are running in `multi` mode
# }

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
    pub port: usize,

    #[serde(default)]
    pub use_self_signed: bool,

    #[serde(default)]
    pub cert_file: Option<String>,

    #[serde(default)]
    pub key_file: Option<String>,
}

impl Https {
    fn default_port() -> usize {
        443
    }

    fn default_status() -> bool {
        false
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Single,
    Multi,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Root {
    Path(String),

    Config {
        path: String,

        #[serde(default = "Root::default_ignore_matches")]
        ignore_matches: Vec<String>,
    },
}

impl Root {
    fn default_ignore_matches() -> Vec<String> {
        vec![]
    }

    pub fn is_empty(&self) -> bool {
        return self.get_path().is_empty();
    }

    pub fn set_path(&mut self, path_str: &str) -> &Self {
        match self {
            Root::Path(path) => *path = path_str.into(),
            Root::Config { path, .. } => *path = path_str.into(),
        }

        self
    }

    pub fn get_path(&self) -> &str {
        return match self {
            Root::Path(path) => path.as_ref(),
            Root::Config { path, .. } => path.as_ref(),
        };
    }

    pub fn get_ignore_matches(&self) -> Option<Vec<String>> {
        match self {
            Root::Path(_) => None,
            Root::Config { ignore_matches, .. } => Some(ignore_matches.clone()),
        }
    }
}

impl From<String> for Root {
    fn from(val: String) -> Self {
        Root::Path(val)
    }
}

impl AsRef<Path> for Root {
    fn as_ref(&self) -> &Path {
        Path::new(self.get_path())
    }
}

impl std::fmt::Display for Root {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Root::Path(path) => f.write_str(path.as_str()),
            Root::Config { path, .. } => f.write_str(path.as_str()),
        }
    }
}

impl From<Root> for PathBuf {
    fn from(val: Root) -> Self {
        PathBuf::from(val.get_path())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "Config::default_host")]
    pub host: IpAddr,

    #[serde(default = "Config::default_port")]
    pub port: usize,

    #[serde(default)]
    pub domain_names: Vec<String>,

    #[serde(default = "Config::default_mode")]
    pub mode: Mode,

    #[serde(default = "Config::default_logging_flag")]
    pub enable_logging: bool,

    #[serde(default = "Config::default_root")]
    pub root: Root,

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
    pub fn default_host() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
    }

    pub fn default_port() -> usize {
        80
    }

    pub fn default_logging_flag() -> bool {
        true
    }

    pub fn default_root() -> Root {
        Root::Path("public".to_string())
    }

    pub fn default_mode() -> Mode {
        Mode::Single
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

pub fn parse_config(config_path: &Path, raw_config: String) -> Result<Config, ChimneyError> {
    let mut config: Config =
        toml::from_str(&raw_config).map_err(|e| InvalidConfig(e.message().to_string()))?;

    if config.root.is_empty() {
        return Err(RootDirNotSet);
    }

    // Expand the root directory to an absolute path
    let current_dir = std::env::current_dir().map_err(FailedToGetWorkingDir)?;

    let parent_dir = if let Some(parent) = config_path.parent() {
        parent
    } else {
        current_dir.as_path()
    };

    let root = absolute_path_str!(parent_dir.join(&config.root));
    config.root.set_path(root.as_str());

    Ok(config)
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

    parse_config(config_path, raw_config)
}
