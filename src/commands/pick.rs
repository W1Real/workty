use crate::git::GitRepo;
use crate::status::get_all_statuses;
use crate::ui::{print_error, UiOptions};
use crate::worktree::list_worktrees;
use anyhow::Result;
use console::Term;
use dialoguer::FuzzySelect;
use is_terminal::IsTerminal;

pub fn execute(repo: &GitRepo, _opts: &UiOptions) -> Result<()> {
    if !std::io::stdin().is_terminal() {
        print_error(
            "Cannot run interactive picker in non-TTY environment",
            Some("Use `git workty go <name>` for non-interactive selection."),
        );
        std::process::exit(1);
    }

    let worktrees = list_worktrees(repo)?;
    if worktrees.is_empty() {
        print_error("No worktrees found", None);
        std::process::exit(1);
    }

    // Get status for richer display
    let statuses = get_all_statuses(repo, &worktrees);

    // Find max name length for alignment
    let max_name_len = statuses
        .iter()
        .map(|(wt, _)| wt.name().len())
        .max()
        .unwrap_or(10);

    let items: Vec<String> = statuses
        .iter()
        .map(|(wt, status)| {
            let name = format!("{:width$}", wt.name(), width = max_name_len);

            // Dirty indicator
            let dirty = if status.dirty_count > 0 {
                format!("*{}", status.dirty_count)
            } else {
                " ".to_string()
            };

            // Time since last commit
            let time = format_time_short(status.last_commit_time);

            // Needs rebase indicator
            let rebase = if status.needs_rebase() {
                format!("R{}", status.behind_main.unwrap_or(0))
            } else {
                "  ".to_string()
            };

            format!("{}  {:>3}  {:>4}  {}", name, dirty, time, rebase)
        })
        .collect();

    let selection = FuzzySelect::new()
        .with_prompt("Select worktree")
        .items(&items)
        .default(0)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(idx) => {
            println!("{}", statuses[idx].0.path.display());
            Ok(())
        }
        None => {
            std::process::exit(130);
        }
    }
}

fn format_time_short(seconds: Option<i64>) -> String {
    match seconds {
        Some(s) if s < 60 => "now".to_string(),
        Some(s) if s < 3600 => format!("{}m", s / 60),
        Some(s) if s < 86400 => format!("{}h", s / 3600),
        Some(s) if s < 604800 => format!("{}d", s / 86400),
        Some(s) => format!("{}w", s / 604800),
        None => "-".to_string(),
    }
}
