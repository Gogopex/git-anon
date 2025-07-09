pub mod anonymize;
pub mod config;
pub mod git;

use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AnonymousIdentity {
    pub name: String,
    pub email: String,
}

impl Default for AnonymousIdentity {
    fn default() -> Self {
        Self {
            name: "Anonymous".to_string(),
            email: "anonymous@example.com".to_string(),
        }
    }
}

pub struct GitAnon {
    pub repo_path: std::path::PathBuf,
    pub identity: AnonymousIdentity,
}

impl GitAnon {
    pub fn new<P: AsRef<Path>>(repo_path: P, identity: AnonymousIdentity) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();

        if !repo_path.join(".git").exists() {
            anyhow::bail!("Not a git repository: {}", repo_path.display());
        }

        Ok(Self {
            repo_path,
            identity,
        })
    }
}
