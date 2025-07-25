use anyhow::Result;
use git2::{BranchType, Commit, Oid, Repository, Signature, Status, StatusOptions};
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
        let mut opts = StatusOptions::new();
        opts.include_untracked(false).include_ignored(false);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let change_flags = Status::INDEX_MODIFIED
            | Status::INDEX_NEW
            | Status::INDEX_DELETED
            | Status::INDEX_RENAMED
            | Status::INDEX_TYPECHANGE
            | Status::WT_MODIFIED
            | Status::WT_DELETED
            | Status::WT_RENAMED
            | Status::WT_TYPECHANGE;

        Ok(statuses
            .iter()
            .any(|status| status.status().intersects(change_flags)))
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

        let commits = self.collect_commits(since_commit)?;
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

        pb.finish_with_message(format!("Anonymized {total} commits"));
        Ok(total)
    }

    pub fn push_to_remote(&self, remote_name: &str, branch: &str, force: bool) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;
        let refspec = format!(
            "{}refs/heads/{}:refs/heads/{}",
            if force { "+" } else { "" },
            branch,
            branch
        );

        remote.push(&[&refspec], None)?;
        Ok(())
    }

    pub fn get_remote_tracking_branch(&self, remote: &str, branch: &str) -> Result<Option<Oid>> {
        let refname = format!("refs/remotes/{remote}/{branch}");
        match self.repo.find_reference(&refname) {
            Ok(reference) => Ok(reference.target()),
            Err(_) => Ok(None),
        }
    }

    pub fn count_commits_to_anonymize(&self, since_commit: Option<&str>) -> Result<u32> {
        Ok(self.collect_commits(since_commit)?.len() as u32)
    }

    fn collect_commits(&self, since_commit: Option<&str>) -> Result<Vec<Oid>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;

        if let Some(since) = since_commit {
            let oid = self.repo.revparse_single(since)?.id();
            revwalk.hide(oid)?;
        }

        Ok(revwalk.collect::<Result<Vec<_>, _>>()?)
    }
}
