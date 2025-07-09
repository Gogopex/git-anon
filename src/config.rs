use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::AnonymousIdentity;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub anonymous_identity: Identity,
    #[serde(default)]
    pub remotes: HashMap<String, RemoteConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub name: String,
    pub identity: String,
}

impl Default for Identity {
    fn default() -> Self {
        Self {
            name: "Anonymous".to_string(),
            email: "anonymous@example.com".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut remotes = HashMap::new();
        remotes.insert(
            "radicle".to_string(),
            RemoteConfig {
                name: "rad".to_string(),
                identity: "anonymous_identity".to_string(),
            },
        );

        Self {
            anonymous_identity: Identity::default(),
            remotes,
        }
    }
}

impl Config {
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("git-anon");

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        Ok(config_dir.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            let default_config = Self::default();
            default_config.save()?;
            return Ok(default_config);
        }

        let contents = fs::read_to_string(&config_path).context("Failed to read config file")?;

        toml::from_str(&contents).context("Failed to parse config file")
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let contents = toml::to_string_pretty(self)?;
        fs::write(config_path, contents)?;
        Ok(())
    }

    pub fn get_identity(&self, name: &str) -> Option<AnonymousIdentity> {
        (name == "anonymous_identity").then(|| AnonymousIdentity {
            name: self.anonymous_identity.name.clone(),
            email: self.anonymous_identity.email.clone(),
        })
    }

    pub fn get_remote_identity(&self, remote: &str) -> AnonymousIdentity {
        self.remotes
            .get(remote)
            .and_then(|rc| self.get_identity(&rc.identity))
            .unwrap_or_else(|| AnonymousIdentity {
                name: self.anonymous_identity.name.clone(),
                email: self.anonymous_identity.email.clone(),
            })
    }
}
