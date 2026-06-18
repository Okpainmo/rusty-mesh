//! # Configuration Management
//!
//! Loads application configuration from base TOML, environment-specific TOML,
//! optional local overrides, and `APP__` environment variables.

use anyhow::{Context, Result};
use config::{Config, Environment, File};
use serde::Deserialize;
use std::fmt;

/// Application-specific metadata section.
#[derive(Clone, Debug, Deserialize)]
pub struct AppSection {
    /// The name of the application.
    pub name: String,
    /// The current environment.
    pub environment: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClientIntegrationsSection {
    #[serde(default)]
    pub allow_logging_middleware: bool,

    #[serde(default)]
    pub allow_request_timeout_middleware: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerSection {
    pub host: String,
    pub port: u16,
    pub request_timeout_secs: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RegistrySection {
    pub heartbeat_interval_secs: u64,
    pub service_ttl_secs: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SecuritySection {
    pub require_mesh_token: bool,
    pub mesh_token: Option<String>,
}

/// Root configuration structure containing all application settings.
#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub app: AppSection,
    pub client_integrations: ClientIntegrationsSection,
    pub server: ServerSection,
    pub registry: RegistrySection,
    pub security: SecuritySection,
}

/// Loads the application configuration.
///
/// Order of precedence, highest to lowest:
/// 1. `APP__` environment variables
/// 2. `config/local.toml`
/// 3. `config/{APP__ENV}.toml`
/// 4. `config/base.toml`
pub fn load_config() -> Result<AppConfig> {
    let env = std::env::var("APP__ENV").context(
        "APP__ENV environment variable is not set! Please set it to 'development', 'production', etc.",
    )?;

    Config::builder()
        .add_source(File::with_name("config/base").required(true))
        .add_source(File::with_name(&format!("config/{}", env)).required(false))
        .add_source(File::with_name("config/local").required(false))
        .add_source(
            Environment::default()
                .separator("__")
                .prefix("APP")
                .try_parsing(true),
        )
        .build()
        .context("Failed to build config")?
        .try_deserialize()
        .context("Invalid config shape")
}

#[derive(Debug)]
pub enum ConfigError {
    MissingAppName,
    MissingServerHost,
    InvalidServerPort,
    InvalidRequestTimeout,
    InvalidHeartbeatInterval,
    InvalidRegistryTtl,
    InvalidRegistryHeartbeatTtlPair,
    MissingMeshToken,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingAppName => write!(f, "app.name cannot be empty"),
            ConfigError::MissingServerHost => write!(f, "server.host cannot be empty"),
            ConfigError::InvalidServerPort => write!(f, "server.port cannot be 0"),
            ConfigError::InvalidRequestTimeout => {
                write!(f, "server.request_timeout_secs cannot be 0")
            }
            ConfigError::InvalidHeartbeatInterval => {
                write!(f, "registry.heartbeat_interval_secs cannot be 0")
            }
            ConfigError::InvalidRegistryTtl => write!(f, "registry.service_ttl_secs cannot be 0"),
            ConfigError::InvalidRegistryHeartbeatTtlPair => write!(
                f,
                "registry.heartbeat_interval_secs must be lower than registry.service_ttl_secs"
            ),
            ConfigError::MissingMeshToken => write!(
                f,
                "security.mesh_token cannot be empty when security.require_mesh_token is true"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl AppConfig {
    pub fn validate(&self) -> std::result::Result<(), ConfigError> {
        if self.app.name.trim().is_empty() {
            return Err(ConfigError::MissingAppName);
        }
        if self.server.host.trim().is_empty() {
            return Err(ConfigError::MissingServerHost);
        }
        if self.server.port == 0 {
            return Err(ConfigError::InvalidServerPort);
        }
        if self.server.request_timeout_secs == 0 {
            return Err(ConfigError::InvalidRequestTimeout);
        }
        if self.registry.heartbeat_interval_secs == 0 {
            return Err(ConfigError::InvalidHeartbeatInterval);
        }
        if self.registry.service_ttl_secs == 0 {
            return Err(ConfigError::InvalidRegistryTtl);
        }
        if self.registry.heartbeat_interval_secs >= self.registry.service_ttl_secs {
            return Err(ConfigError::InvalidRegistryHeartbeatTtlPair);
        }
        if self.security.require_mesh_token
            && self
                .security
                .mesh_token
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(ConfigError::MissingMeshToken);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> AppConfig {
        AppConfig {
            app: AppSection {
                name: "mesh_service".to_string(),
                environment: Some("test".to_string()),
            },
            client_integrations: ClientIntegrationsSection {
                allow_logging_middleware: true,
                allow_request_timeout_middleware: true,
            },
            server: ServerSection {
                host: "127.0.0.1".to_string(),
                port: 3080,
                request_timeout_secs: 60,
            },
            registry: RegistrySection {
                heartbeat_interval_secs: 5,
                service_ttl_secs: 15,
            },
            security: SecuritySection {
                require_mesh_token: true,
                mesh_token: Some("test-mesh-token".to_string()),
            },
        }
    }

    #[test]
    fn validate_accepts_heartbeat_interval_lower_than_ttl() {
        let config = valid_config();

        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_heartbeat_interval_equal_to_ttl() {
        let mut config = valid_config();
        config.registry.heartbeat_interval_secs = 15;

        let error = config
            .validate()
            .expect_err("heartbeat interval equal to ttl should fail");

        assert!(matches!(
            error,
            ConfigError::InvalidRegistryHeartbeatTtlPair
        ));
    }

    #[test]
    fn validate_rejects_heartbeat_interval_higher_than_ttl() {
        let mut config = valid_config();
        config.registry.heartbeat_interval_secs = 20;

        let error = config
            .validate()
            .expect_err("heartbeat interval higher than ttl should fail");

        assert!(matches!(
            error,
            ConfigError::InvalidRegistryHeartbeatTtlPair
        ));
    }

    #[test]
    fn validate_rejects_missing_mesh_token_when_required() {
        let mut config = valid_config();
        config.security.mesh_token = Some("   ".to_string());

        let error = config
            .validate()
            .expect_err("missing required mesh token should fail");

        assert!(matches!(error, ConfigError::MissingMeshToken));
    }

    #[test]
    fn validate_accepts_missing_mesh_token_when_disabled() {
        let mut config = valid_config();
        config.security.require_mesh_token = false;
        config.security.mesh_token = None;

        assert!(config.validate().is_ok());
    }
}
