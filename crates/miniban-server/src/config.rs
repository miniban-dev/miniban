// Copyright (c) 2026 Braden Hitchcock - AGPL-3.0-only (see LICENSE for details)

//! Runtime configuration read from the environment. [`Config::from_env`] builds a [`Config`] or
//! reports a [`ConfigError`].

use std::env::{self, VarError};
use std::error::Error;
use std::fmt;

/// The bind host used when `MINIBAN_HOST` is unset.
const DEFAULT_HOST: &str = "127.0.0.1";

/// The port used when `MINIBAN_PORT` is unset.
const DEFAULT_PORT: u16 = 3000;

/// Where the server listens.
#[derive(Debug)]
pub struct Config {
    /// The hostname or IP address to bind.
    pub host: String,
    /// The TCP port to bind.
    pub port: u16,
}

impl Config {
    /// Reads `MINIBAN_HOST` and `MINIBAN_PORT`, using defaults for those that are unset.
    pub fn from_env() -> Result<Self, ConfigError> {
        let host = read_var("MINIBAN_HOST")?.unwrap_or_else(|| DEFAULT_HOST.to_owned());
        let port = match read_var("MINIBAN_PORT")? {
            Some(value) => value
                .parse()
                .map_err(|_| ConfigError::InvalidPort { value })?,
            None => DEFAULT_PORT,
        };
        Ok(Self { host, port })
    }
}

/// Reads an environment variable, returning `None` if it is unset.
fn read_var(name: &'static str) -> Result<Option<String>, ConfigError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(ConfigError::NotUnicode { name }),
    }
}

/// An environment variable holds a value the server can't use.
#[derive(Debug)]
pub enum ConfigError {
    /// The named variable isn't valid Unicode.
    NotUnicode {
        /// The variable's name.
        name: &'static str,
    },
    /// `MINIBAN_PORT` isn't a valid TCP port.
    InvalidPort {
        /// The rejected value.
        value: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotUnicode { name } => write!(f, "{name} must be valid Unicode"),
            Self::InvalidPort { value } => {
                write!(
                    f,
                    "MINIBAN_PORT must be a port number from 0 to 65535, got {value:?}"
                )
            }
        }
    }
}

impl Error for ConfigError {}
