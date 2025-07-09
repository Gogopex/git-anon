use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use dialoguer::Confirm;
use std::path::Path;

use crate::git::GitOps;
use crate::{AnonymousIdentity, GitAnon};

impl GitAnon {
    pub fn squash(&self, message: Option<String>, no_confirm: bool) -> Result<()> {
        let git = GitOps::open(&self.repo_path)?;
        let branch = git.current_branch()?;

        if git.has_uncommitted_changes()? {
            anyhow::bail!("Uncommitted changes detected. Please commit or stash them first.");
        }

        let message = message.unwrap_or_else(|| "Initial commit".to_string());

        if !no_confirm {
            println!(
                "{}",
                "WARNING: This will squash ALL commits into a single anonymous commit!"
                    .red()
                    .bold()
            );
            println!("Current branch: {}", branch.yellow());
            println!("New commit message: {}", message.cyan());
            println!(
                "Anonymous identity: {} <{}>",
                self.identity.name, self.identity.email
            );
            println!();

            if !Confirm::new()
                .with_prompt("Continue?")
                .default(false)
                .interact()?
            {
                println!("Aborted.");
                return Ok(());
            }
        }

        let backup_branch = format!("backup-{}-{}", branch, Utc::now().timestamp());
        println!("Creating backup branch: {}", backup_branch.green());
        git.create_backup_branch(&backup_branch)?;

        println!("Squashing all commits...");
        git.squash_all_commits(&self.identity, &message, &branch)?;

        println!("{} Successfully squashed all commits!", "✓".green());
        println!("Backup saved to branch: {}", backup_branch.yellow());

        Ok(())
    }

    pub fn push(&self, remote: &str, branch: Option<String>, force: bool) -> Result<()> {
        let git = GitOps::open(&self.repo_path)?;
        let current_branch = git.current_branch()?;
        let branch = branch.unwrap_or(current_branch);

        if git.has_uncommitted_changes()? {
            anyhow::bail!("Uncommitted changes detected. Please commit or stash them first.");
        }

        println!("Checking for commits to anonymize...");

        let remote_oid = git.get_remote_tracking_branch(remote, &branch)?;
        let since_commit = remote_oid.map(|oid| oid.to_string());

        let count = git.anonymize_commits(&self.identity, &branch, since_commit.as_deref())?;

        if count == 0 {
            println!("Already up to date with {}/{}", remote, branch);
            return Ok(());
        }

        println!("Pushing to {}...", remote);
        git.push_to_remote(remote, &branch, force)?;

        println!(
            "{} Successfully pushed {} anonymized commits to {}",
            "✓".green(),
            count,
            remote
        );

        Ok(())
    }

    pub fn clean(&self, no_confirm: bool) -> Result<()> {
        let git = GitOps::open(&self.repo_path)?;

        if !no_confirm {
            println!(
                "{}",
                "WARNING: This will COMPLETELY ANONYMIZE the repository!"
                    .red()
                    .bold()
            );
            println!("This includes:");
            println!("  - Squashing all commits into one");
            println!("  - Removing all git history");
            println!("  - Removing git submodules");
            println!("  - Cleaning git reflog");
            println!();

            if !Confirm::new()
                .with_prompt("This action is IRREVERSIBLE. Continue?")
                .default(false)
                .interact()?
            {
                println!("Aborted.");
                return Ok(());
            }
        }

        let branch = git.current_branch()?;
        let backup_branch = format!("pre-clean-backup-{}", Utc::now().timestamp());

        println!("Creating final backup branch: {}", backup_branch.green());
        git.create_backup_branch(&backup_branch)?;

        println!("Squashing all commits...");
        git.squash_all_commits(&self.identity, "Initial commit", &branch)?;

        println!("Cleaning git history...");
        std::process::Command::new("git")
            .arg("-C")
            .arg(&self.repo_path)
            .args(&["reflog", "expire", "--expire=now", "--all"])
            .output()?;

        std::process::Command::new("git")
            .arg("-C")
            .arg(&self.repo_path)
            .args(&["gc", "--prune=now", "--aggressive"])
            .output()?;

        println!("{} Repository fully anonymized!", "✓".green());
        println!("Backup saved to branch: {}", backup_branch.yellow());

        Ok(())
    }
}
