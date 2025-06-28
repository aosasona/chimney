use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// Represents the available log levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Off = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl Default for LogLevel {
    fn default() -> Self {
        if cfg!(debug_assertions) {
            LogLevel::Trace
        } else {
            LogLevel::Info
        }
    }
}

impl LogLevel {
    /// Converts the log level to a `log::LevelFilter`
    pub fn to_log_level_filter(&self) -> log::LevelFilter {
        match self {
            LogLevel::Off => log::LevelFilter::Off,
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }

    /// Checks if the current log level is lower than the provided log level
    pub fn is_higher_than(&self, other: &LogLevel) -> bool {
        self.to_log_level_filter() > other.to_log_level_filter()
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(LogLevel::Off),
            "error" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(format!("Invalid log level: {s}")),
        }
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level_str = match self {
            LogLevel::Off => "off",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        };
        write!(f, "{level_str}")
    }
}
