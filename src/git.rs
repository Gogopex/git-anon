use anyhow::{Context, Result};
use git2::{BranchType, Commit, ObjectType, Oid, Reference, Repository, Signature, Time};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

use crate::AnonymousIdentity;

pub struct GitOps {
    repo: Repository,
}

impl GitOps {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let repo = Repository::open(path)?;
        Ok(Self { repo })
    }

    pub fn current_branch(&self) -> Result<String> {
        let head = self.repo.head()?;
        let shorthand = head.shorthand().unwrap_or("HEAD");
        Ok(shorthand.to_string())
    }

    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        let statuses = self.repo.statuses(None)?;
        Ok(statuses.len() > 0)
    }

    pub fn create_backup_branch(&self, branch_name: &str) -> Result<()> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        self.repo.branch(branch_name, &commit, false)?;
        Ok(())
    }

    pub fn squash_all_commits(
        &self,
        identity: &AnonymousIdentity,
        message: &str,
        branch: &str,
    ) -> Result<()> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::REVERSE)?;

        let commits: Vec<Oid> = revwalk.collect::<Result<Vec<_>, _>>()?;
        if commits.is_empty() {
            anyhow::bail!("No commits found in repository");
        }

        let first_commit = self.repo.find_commit(commits[0])?;
        let tree = self
            .repo
            .find_commit(self.repo.head()?.target().unwrap())?
            .tree()?;

        let signature = Signature::now(&identity.name, &identity.email)?;

        let new_commit_oid = self
            .repo
            .commit(None, &signature, &signature, message, &tree, &[])?;

        let mut branch_ref = self.repo.find_branch(branch, BranchType::Local)?;
        branch_ref
            .get_mut()
            .set_target(new_commit_oid, "Squashed all commits")?;

        Ok(())
    }

    pub fn anonymize_commits(
        &self,
        identity: &AnonymousIdentity,
        branch: &str,
        since_commit: Option<&str>,
    ) -> Result<u32> {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Collecting commits to anonymize...");

        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;

        if let Some(since) = since_commit {
            let oid = self.repo.revparse_single(since)?.id();
            revwalk.hide(oid)?;
        }

        let commits: Vec<Oid> = revwalk.collect::<Result<Vec<_>, _>>()?;
        let total = commits.len() as u32;

        if total == 0 {
            pb.finish_with_message("No commits to anonymize");
            return Ok(0);
        }

        pb.set_length(total as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{bar:40.cyan/blue} {pos}/{len} {msg}")
                .unwrap(),
        );

        let signature = Signature::now(&identity.name, &identity.email)?;
        let mut new_commits = std::collections::HashMap::new();

        for (i, &oid) in commits.iter().rev().enumerate() {
            pb.set_position(i as u64);
            pb.set_message(format!("Anonymizing commit {}", &oid.to_string()[..8]));

            let commit = self.repo.find_commit(oid)?;
            let tree = commit.tree()?;

            let new_parents: Vec<Commit> = commit
                .parent_ids()
                .filter_map(|pid| {
                    new_commits
                        .get(&pid)
                        .and_then(|&new_oid| self.repo.find_commit(new_oid).ok())
                })
                .collect();

            let parents_refs: Vec<&Commit> = new_parents.iter().collect();

            let new_oid = self.repo.commit(
                None,
                &signature,
                &signature,
                commit.message().unwrap_or(""),
                &tree,
                &parents_refs,
            )?;

            new_commits.insert(oid, new_oid);
        }

        if let Some(&new_head) = new_commits.get(&commits[0]) {
            let mut branch_ref = self.repo.find_branch(branch, BranchType::Local)?;
            branch_ref
                .get_mut()
                .set_target(new_head, "Anonymized commits")?;
        }

        pb.finish_with_message(format!("Anonymized {} commits", total));
        Ok(total)
    }

    pub fn push_to_remote(&self, remote_name: &str, branch: &str, force: bool) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;
        let refspec = if force {
            format!("+refs/heads/{}:refs/heads/{}", branch, branch)
        } else {
            format!("refs/heads/{}:refs/heads/{}", branch, branch)
        };

        remote.push(&[&refspec], None)?;
        Ok(())
    }

    pub fn get_remote_tracking_branch(&self, remote: &str, branch: &str) -> Result<Option<Oid>> {
        let refname = format!("refs/remotes/{}/{}", remote, branch);
        match self.repo.find_reference(&refname) {
            Ok(reference) => Ok(reference.target()),
            Err(_) => Ok(None),
        }
    }
}
