//! Configuration for binostr benchmarks and analysis
//!
//! Loads settings from `binostr.toml` in the project root to control
//! which formats are included in benchmarks and comparisons.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;

/// Global config instance, loaded once on first access
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Configuration structure matching binostr.toml
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub formats: FormatConfig,
}

/// Per-format enable/disable settings
#[derive(Debug, Clone, Deserialize)]
pub struct FormatConfig {
    #[serde(default = "default_true")]
    pub json: bool,

    #[serde(default = "default_true")]
    pub cbor_schemaless: bool,

    #[serde(default = "default_true")]
    pub cbor_packed: bool,

    #[serde(default = "default_true")]
    pub cbor_intkey: bool,

    #[serde(default = "default_true")]
    pub msgpack: bool,

    #[serde(default = "default_true")]
    pub flatbuffers: bool,

    #[serde(default = "default_true")]
    pub flexbuffers: bool,

    #[serde(default = "default_true")]
    pub avro: bool,

    #[serde(default = "default_true")]
    pub bson: bool,

    #[serde(default = "default_true")]
    pub thrift_compact: bool,

    #[serde(default = "default_true")]
    pub beve: bool,

    #[serde(default = "default_true")]
    pub bincode: bool,

    #[serde(default = "default_true")]
    pub postcard: bool,

    #[serde(default = "default_true")]
    pub proto_string: bool,

    #[serde(default = "default_true")]
    pub proto_binary: bool,

    #[serde(default = "default_true")]
    pub capnp: bool,

    #[serde(default = "default_true")]
    pub capnp_packed: bool,

    #[serde(default = "default_true")]
    pub dannypack: bool,

    #[serde(default = "default_true")]
    pub notepack: bool,
}

fn default_true() -> bool {
    true
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            json: true,
            cbor_schemaless: true,
            cbor_packed: true,
            cbor_intkey: true,
            msgpack: true,
            flatbuffers: true,
            flexbuffers: true,
            avro: true,
            bson: true,
            thrift_compact: true,
            beve: true,
            bincode: true,
            postcard: true,
            proto_string: true,
            proto_binary: true,
            capnp: true,
            capnp_packed: true,
            dannypack: true,
            notepack: true,
        }
    }
}

impl Config {
    /// Load config from the given path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path.as_ref()).map_err(|e| ConfigError::Io {
            path: path.as_ref().display().to_string(),
            error: e,
        })?;

        toml::from_str(&contents).map_err(|e| ConfigError::Parse {
            path: path.as_ref().display().to_string(),
            error: e,
        })
    }

    /// Try to load config from standard locations, falling back to defaults
    pub fn load() -> Self {
        // Try these paths in order
        let paths = ["binostr.toml", ".binostr.toml", "config/binostr.toml"];

        for path in paths {
            if Path::new(path).exists() {
                match Self::from_file(path) {
                    Ok(config) => {
                        eprintln!("Loaded config from {}", path);
                        return config;
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load {}: {}", path, e);
                    }
                }
            }
        }

        // No config file found, use defaults (all formats enabled)
        Self::default()
    }

    /// Get the format enable/disable map for easy lookup
    pub fn format_enabled_map(&self) -> HashMap<&'static str, bool> {
        let mut map = HashMap::new();
        map.insert("json", self.formats.json);
        map.insert("cbor_schemaless", self.formats.cbor_schemaless);
        map.insert("cbor_packed", self.formats.cbor_packed);
        map.insert("cbor_intkey", self.formats.cbor_intkey);
        map.insert("msgpack", self.formats.msgpack);
        map.insert("flatbuffers", self.formats.flatbuffers);
        map.insert("flexbuffers", self.formats.flexbuffers);
        map.insert("avro", self.formats.avro);
        map.insert("bson", self.formats.bson);
        map.insert("thrift_compact", self.formats.thrift_compact);
        map.insert("beve", self.formats.beve);
        map.insert("bincode", self.formats.bincode);
        map.insert("postcard", self.formats.postcard);
        map.insert("proto_string", self.formats.proto_string);
        map.insert("proto_binary", self.formats.proto_binary);
        map.insert("capnp", self.formats.capnp);
        map.insert("capnp_packed", self.formats.capnp_packed);
        map.insert("dannypack", self.formats.dannypack);
        map.insert("notepack", self.formats.notepack);
        map
    }
}

/// Get the global config instance
pub fn get() -> &'static Config {
    CONFIG.get_or_init(Config::load)
}

/// Check if a format is enabled by its short name
pub fn is_format_enabled(name: &str) -> bool {
    let config = get();
    match name {
        "json" => config.formats.json,
        "cbor_schemaless" | "cbor_schema" => config.formats.cbor_schemaless,
        "cbor_packed" => config.formats.cbor_packed,
        "cbor_intkey" => config.formats.cbor_intkey,
        "msgpack" => config.formats.msgpack,
        "flatbuffers" => config.formats.flatbuffers,
        "flexbuffers" => config.formats.flexbuffers,
        "avro" => config.formats.avro,
        "bson" => config.formats.bson,
        "thrift_compact" | "thrift" => config.formats.thrift_compact,
        "beve" => config.formats.beve,
        "bincode" => config.formats.bincode,
        "postcard" => config.formats.postcard,
        "proto_string" | "proto_str" => config.formats.proto_string,
        "proto_binary" | "proto_bin" => config.formats.proto_binary,
        "capnp" => config.formats.capnp,
        "capnp_packed" | "capnp_pk" => config.formats.capnp_packed,
        "dannypack" => config.formats.dannypack,
        "notepack" => config.formats.notepack,
        _ => true, // Unknown formats default to enabled
    }
}

/// Configuration loading errors
#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: String,
        error: std::io::Error,
    },
    Parse {
        path: String,
        error: toml::de::Error,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io { path, error } => {
                write!(f, "Failed to read config file '{}': {}", path, error)
            }
            ConfigError::Parse { path, error } => {
                write!(f, "Failed to parse config file '{}': {}", path, error)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.formats.json);
        assert!(config.formats.notepack);
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
            [formats]
            json = true
            cbor_schemaless = false
            notepack = true
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.formats.json);
        assert!(!config.formats.cbor_schemaless);
        assert!(config.formats.notepack);
        // Unspecified formats should default to true
        assert!(config.formats.proto_binary);
    }
}
