use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server_port: u16,
    pub scripts_dir: String,
    pub log_dir: String,
    pub retention_days: u32,
    pub admin_email: String,
    pub smtp: SmtpConfig,
    pub ad_integration: ActiveDirectoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveDirectoryConfig {
    pub enabled: bool,
    pub server: String,
    pub domain: String,
    pub bind_dn: String,
    pub bind_password: String,
}

pub fn default_config() -> Config {
    Config {
        server_port: 8080,
        scripts_dir: "scripts".to_string(),
        log_dir: "logs".to_string(),
        retention_days: 365, // 1 year retention as per regulation
        admin_email: "admin@example.com".to_string(),
        smtp: SmtpConfig {
            server: "smtp.example.com".to_string(),
            port: 587,
            username: "notify@example.com".to_string(),
            password: "change-me".to_string(),
            use_tls: true,
        },
        ad_integration: ActiveDirectoryConfig {
            enabled: false,
            server: "ldap://ad.example.com".to_string(),
            domain: "EXAMPLE".to_string(),
            bind_dn: "cn=siem,ou=Service Accounts,dc=example,dc=com".to_string(),
            bind_password: "change-me".to_string(),
        },
    }
}

pub fn load(config_path: &str) -> Result<Config> {
    let config_str = fs::read_to_string(config_path)
        .context(format!("Failed to read config file: {}", config_path))?;

    let config: Config = toml::from_str(&config_str)
        .context(format!("Failed to parse config file: {}", config_path))?;

    Ok(config)
}

pub fn save(config: &Config, config_path: &str) -> Result<()> {
    let config_str = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;

    if let Some(parent) = Path::new(config_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create directory: {:?}", parent))?;
        }
    }

    fs::write(config_path, config_str)
        .context(format!("Failed to write config file: {}", config_path))?;

    Ok(())
}