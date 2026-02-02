use crate::git::GitRepo;
use crate::status::WorktreeStatus;
use crate::worktree::Worktree;
use owo_colors::OwoColorize;
use serde::Serialize;
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct UiOptions {
    pub color: bool,
    pub ascii: bool,
    pub json: bool,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            color: true,
            ascii: false,
            json: false,
        }
    }
}

pub struct Icons {
    pub current: &'static str,
    pub dirty: &'static str,
    pub clean: &'static str,
    pub arrow_up: &'static str,
    pub arrow_down: &'static str,
    pub rebase: &'static str,
}

impl Icons {
    pub fn unicode() -> Self {
        Self {
            current: "▶",
            dirty: "●",
            clean: "✓",
            arrow_up: "↑",
            arrow_down: "↓",
            rebase: "⟳",
        }
    }

    pub fn ascii() -> Self {
        Self {
            current: ">",
            dirty: "*",
            clean: "-",
            arrow_up: "^",
            arrow_down: "v",
            rebase: "R",
        }
    }

    pub fn from_options(opts: &UiOptions) -> Self {
        if opts.ascii {
            Self::ascii()
        } else {
            Self::unicode()
        }
    }
}

pub fn print_worktree_list(
    repo: &GitRepo,
    worktrees: &[(Worktree, WorktreeStatus)],
    current_path: &Path,
    opts: &UiOptions,
) {
    if opts.json {
        print_worktree_list_json(repo, worktrees, current_path);
        return;
    }

    let icons = Icons::from_options(opts);

    // Calculate column widths
    let max_name_len = worktrees
        .iter()
        .map(|(wt, _)| wt.name().len())
        .max()
        .unwrap_or(10)
        .max(6); // minimum width for "BRANCH" header

    // Print header
    if opts.color {
        println!(
            "  {:width$}  {:>6}  {:>6}  {:>5}  {:>6}  {}",
            "BRANCH".dimmed(),
            "DIRTY".dimmed(),
            "SYNC".dimmed(),
            "AGE".dimmed(),
            "REBASE".dimmed(),
            "PATH".dimmed(),
            width = max_name_len
        );
    } else {
        println!(
            "  {:width$}  {:>6}  {:>6}  {:>5}  {:>6}  {}",
            "BRANCH",
            "DIRTY",
            "SYNC",
            "AGE",
            "REBASE",
            "PATH",
            width = max_name_len
        );
    }

    for (wt, status) in worktrees {
        let is_current = wt.path == current_path;

        let marker = if is_current { icons.current } else { " " };

        let name = wt.name();
        let name_padded = format!("{:width$}", name, width = max_name_len);

        let dirty_str = format_dirty(status, &icons, opts);
        let sync_str = format_sync(status, &icons);
        let time_str = format_time(status.last_commit_time);
        let rebase_str = format_rebase(status, &icons, opts);
        let path_str = shorten_path(&wt.path);

        if opts.color {
            let name_colored = if is_current {
                name_padded.green().bold().to_string()
            } else if status.is_dirty() {
                name_padded.yellow().to_string()
            } else {
                name_padded.to_string()
            };

            let marker_colored = if is_current {
                marker.green().bold().to_string()
            } else {
                marker.to_string()
            };

            println!(
                "{} {}  {:>6}  {:>6}  {:>5}  {:>6}  {}",
                marker_colored,
                name_colored,
                dirty_str,
                sync_str,
                time_str.dimmed(),
                rebase_str,
                path_str.dimmed()
            );
        } else {
            println!(
                "{} {}  {:>6}  {:>6}  {:>5}  {:>6}  {}",
                marker, name_padded, dirty_str, sync_str, time_str, rebase_str, path_str
            );
        }
    }
}

fn format_dirty(status: &WorktreeStatus, icons: &Icons, opts: &UiOptions) -> String {
    if status.dirty_count > 0 {
        let s = format!("{} {:>3}", icons.dirty, status.dirty_count);
        if opts.color {
            s.yellow().to_string()
        } else {
            s
        }
    } else {
        let s = format!("{} {:>3}", icons.clean, "-");
        if opts.color {
            s.green().to_string()
        } else {
            s
        }
    }
}

fn format_sync(status: &WorktreeStatus, icons: &Icons) -> String {
    match (status.ahead, status.behind) {
        (Some(a), Some(b)) => {
            format!("{}{} {}{}", icons.arrow_up, a, icons.arrow_down, b)
        }
        _ => "  -  ".to_string(),
    }
}

pub fn format_time(seconds: Option<i64>) -> String {
    match seconds {
        Some(s) if s < 60 => "now".to_string(),
        Some(s) if s < 3600 => format!("{}m", s / 60),
        Some(s) if s < 86400 => format!("{}h", s / 3600),
        Some(s) if s < 604800 => format!("{}d", s / 86400),
        Some(s) if s < 2592000 => format!("{}w", s / 604800),
        Some(s) => format!("{}mo", s / 2592000),
        None => "-".to_string(),
    }
}

fn format_rebase(status: &WorktreeStatus, icons: &Icons, opts: &UiOptions) -> String {
    if let Some(n) = status.behind_main {
        if n > 0 {
            let s = format!("{} {:>3}", icons.rebase, n);
            if opts.color {
                s.red().to_string()
            } else {
                s
            }
        } else {
            "    -".to_string()
        }
    } else {
        "    -".to_string()
    }
}

pub fn shorten_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

#[derive(Serialize)]
struct JsonOutput {
    repo: RepoInfo,
    current: String,
    worktrees: Vec<JsonWorktree>,
}

#[derive(Serialize)]
struct RepoInfo {
    root: String,
    common_dir: String,
}

#[derive(Serialize)]
struct JsonWorktree {
    path: String,
    branch: Option<String>,
    branch_short: Option<String>,
    head: String,
    detached: bool,
    locked: bool,
    dirty_count: usize,
    upstream: Option<String>,
    ahead: Option<usize>,
    behind: Option<usize>,
    last_commit_seconds: Option<i64>,
    behind_main: Option<usize>,
}

fn print_worktree_list_json(
    repo: &GitRepo,
    worktrees: &[(Worktree, WorktreeStatus)],
    current_path: &Path,
) {
    let json_worktrees: Vec<JsonWorktree> = worktrees
        .iter()
        .map(|(wt, status)| JsonWorktree {
            path: wt.path.to_string_lossy().into_owned(),
            branch: wt.branch.clone(),
            branch_short: wt.branch_short.clone(),
            head: wt.head.clone(),
            detached: wt.detached,
            locked: wt.locked,
            dirty_count: status.dirty_count,
            upstream: status.upstream.clone(),
            ahead: status.ahead,
            behind: status.behind,
            last_commit_seconds: status.last_commit_time,
            behind_main: status.behind_main,
        })
        .collect();

    let output = JsonOutput {
        repo: RepoInfo {
            root: repo.root.to_string_lossy().into_owned(),
            common_dir: repo.common_dir.to_string_lossy().into_owned(),
        },
        current: current_path.to_string_lossy().into_owned(),
        worktrees: json_worktrees,
    };

    let json = serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string());
    println!("{}", json);
}

pub fn print_error(msg: &str, hint: Option<&str>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();

    let _ = writeln!(handle, "{}: {}", "error".red().bold(), msg);
    if let Some(h) = hint {
        let _ = writeln!(handle, "{}: {}", "hint".cyan(), h);
    }
}

pub fn print_success(msg: &str) {
    eprintln!("{}: {}", "success".green().bold(), msg);
}

pub fn print_warning(msg: &str) {
    eprintln!("{}: {}", "warning".yellow().bold(), msg);
}

pub fn print_info(msg: &str) {
    eprintln!("{}", msg);
}
