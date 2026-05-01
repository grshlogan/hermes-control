use hermes_control_types::{ControlConfig, ModelRuntimesConfig, ProvidersConfig};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid TOML config: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("LAN bind is disabled by policy but daemon.bind is {0}")]
    LanBindDisallowed(String),
    #[error("raw shell is disabled for Hermes Control")]
    RawShellAllowed,
}

pub fn parse_control_config(input: &str) -> Result<ControlConfig, ConfigError> {
    let config = toml::from_str::<ControlConfig>(input)?;
    validate_control_config(&config)?;
    Ok(config)
}

pub fn parse_providers_config(input: &str) -> Result<ProvidersConfig, ConfigError> {
    Ok(toml::from_str(input)?)
}

pub fn parse_model_runtimes_config(input: &str) -> Result<ModelRuntimesConfig, ConfigError> {
    Ok(toml::from_str(input)?)
}

pub fn validate_control_config(config: &ControlConfig) -> Result<(), ConfigError> {
    if !config.policy.allow_lan_bind && !is_local_bind(&config.daemon.bind) {
        return Err(ConfigError::LanBindDisallowed(config.daemon.bind.clone()));
    }

    if config.policy.allow_raw_shell {
        return Err(ConfigError::RawShellAllowed);
    }

    Ok(())
}

fn is_local_bind(bind: &str) -> bool {
    bind.starts_with("127.") || bind.starts_with("localhost:") || bind.starts_with("[::1]:")
}
