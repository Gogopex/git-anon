use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use dialoguer::Confirm;

use crate::GitAnon;
use crate::git::GitOps;

impl GitAnon {
    pub fn squash(&self, message: Option<String>, no_confirm: bool, dry_run: bool) -> Result<()> {
        let git = GitOps::open(&self.repo_path)?;
        let branch = git.current_branch()?;

        if git.has_uncommitted_changes()? {
            anyhow::bail!("Uncommitted changes detected. Please commit or stash them first.");
        }

        let message = message.unwrap_or_else(|| "Initial commit".to_string());

        if dry_run {
            let backup_branch = format!("backup-{}-{}", branch, Utc::now().timestamp());
            println!("{}", "[DRY RUN] Squash operation preview:".blue().bold());
            println!("  Current branch: {}", branch.yellow());
            println!("  New commit message: {}", message.cyan());
            println!(
                "  Anonymous identity: {} <{}>",
                self.identity.name, self.identity.email
            );
            println!("  Backup branch name: {}", backup_branch.green());
            println!(
                "  {} All commits would be squashed into a single anonymous commit",
                "→".blue()
            );
            println!("  {} A backup branch would be created", "→".blue());
            return Ok(());
        }

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

    pub fn push(
        &self,
        remote: &str,
        branch: Option<String>,
        force: bool,
        dry_run: bool,
    ) -> Result<()> {
        let git = GitOps::open(&self.repo_path)?;
        let current_branch = git.current_branch()?;
        let branch = branch.unwrap_or(current_branch);

        if git.has_uncommitted_changes()? {
            anyhow::bail!("Uncommitted changes detected. Please commit or stash them first.");
        }

        println!("Checking for commits to anonymize...");

        let remote_oid = git.get_remote_tracking_branch(remote, &branch)?;
        let since_commit = remote_oid.map(|oid| oid.to_string());
        let count = git.count_commits_to_anonymize(since_commit.as_deref())?;

        if count == 0 {
            println!("Already up to date with {}/{}", remote, branch);
            return Ok(());
        }

        if dry_run {
            println!("{}", "[DRY RUN] Push operation preview:".blue().bold());
            println!("  Target remote: {}", remote.yellow());
            println!("  Target branch: {}", branch.yellow());
            println!(
                "  Force push: {}",
                if force { "yes".red() } else { "no".green() }
            );
            println!(
                "  Anonymous identity: {} <{}>",
                self.identity.name, self.identity.email
            );
            println!("  {} {} commits would be anonymized", "→".blue(), count);
            println!(
                "  {} Commits would be pushed to {}/{}",
                "→".blue(),
                remote,
                branch
            );
            return Ok(());
        }

        let anonymized_count = git.anonymize_commits(&self.identity, &branch, since_commit.as_deref())?;

        println!("Pushing to {}...", remote);
        git.push_to_remote(remote, &branch, force)?;

        println!(
            "{} Successfully pushed {} anonymized commits to {}",
            "✓".green(),
            anonymized_count,
            remote
        );

        Ok(())
    }

    pub fn clean(&self, no_confirm: bool, dry_run: bool) -> Result<()> {
        let git = GitOps::open(&self.repo_path)?;

        if dry_run {
            println!("{}", "[DRY RUN] Clean operation preview:".blue().bold());
            let branch = git.current_branch()?;
            let backup_branch = format!("pre-clean-backup-{}", Utc::now().timestamp());
            println!("  Current branch: {}", branch.yellow());
            println!("  Backup branch name: {}", backup_branch.green());
            println!(
                "  Anonymous identity: {} <{}>",
                self.identity.name, self.identity.email
            );
            println!("  {} All commits would be squashed into one", "→".blue());
            println!("  {} All git history would be removed", "→".blue());
            println!("  {} Git submodules would be removed", "→".blue());
            println!("  {} Git reflog would be cleaned", "→".blue());
            println!(
                "  {} Aggressive garbage collection would be performed",
                "→".blue()
            );
            println!("  {} A backup branch would be created", "→".blue());
            println!("  {}", "WARNING: This would be IRREVERSIBLE!".red().bold());
            return Ok(());
        }

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
        let cleanup_commands = [
            &["reflog", "expire", "--expire=now", "--all"] as &[&str],
            &["gc", "--prune=now", "--aggressive"] as &[&str],
        ];
        
        for cmd in cleanup_commands {
            std::process::Command::new("git")
                .arg("-C")
                .arg(&self.repo_path)
                .args(cmd)
                .output()?;
        }

        println!("{} Repository fully anonymized!", "✓".green());
        println!("Backup saved to branch: {}", backup_branch.yellow());

        Ok(())
    }
}
