use crate::git::GitRepo;
use crate::worktree::{find_worktree, list_worktrees};
use anyhow::{bail, Result};

pub fn execute(repo: &GitRepo, name: &str) -> Result<()> {
    let worktrees = list_worktrees(repo)?;

    if let Some(wt) = find_worktree(&worktrees, name) {
        println!("{}", wt.path.display());
        Ok(())
    } else {
        bail!(
            "Worktree '{}' not found. Use `git workty list` to see available worktrees.",
            name
        );
    }
}
