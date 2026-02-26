use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitFileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Conflicted,
    Ignored,
}

impl GitFileStatus {
    pub fn label(self) -> &'static str {
        match self {
            GitFileStatus::Added => "A",
            GitFileStatus::Modified => "M",
            GitFileStatus::Deleted => "D",
            GitFileStatus::Renamed => "R",
            GitFileStatus::Copied => "C",
            GitFileStatus::Untracked => "?",
            GitFileStatus::Conflicted => "U",
            GitFileStatus::Ignored => "!",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitRepoStatus {
    pub branch: String,
    pub repo_root: PathBuf,
    pub file_statuses: HashMap<PathBuf, GitFileStatus>,
}

impl GitRepoStatus {
    pub fn change_count(&self) -> usize {
        self.file_statuses
            .values()
            .filter(|status| **status != GitFileStatus::Ignored)
            .count()
    }
}

pub fn git_repo_status_for_path(path: &Path) -> Option<GitRepoStatus> {
    let repo_root = git_repo_root(path)?;

    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("status")
        .arg("--porcelain=1")
        .arg("--ignored=matching")
        .arg("-b")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let status = String::from_utf8(output.stdout).ok()?;
    parse_porcelain_status(&status, repo_root)
}

fn git_repo_root(path: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let root = String::from_utf8(output.stdout).ok()?;
    let trimmed = root.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn parse_porcelain_status(text: &str, repo_root: PathBuf) -> Option<GitRepoStatus> {
    let mut lines = text.lines();
    let head_line = lines.next()?;
    if !head_line.starts_with("## ") {
        return None;
    }

    let branch = parse_branch_name(head_line)?;
    let mut file_statuses = HashMap::new();

    for line in lines {
        if line.len() < 3 {
            continue;
        }

        if line.starts_with("?? ") {
            let rel = PathBuf::from(line[3..].trim());
            file_statuses.insert(rel, GitFileStatus::Untracked);
            continue;
        }
        if line.starts_with("!! ") {
            let rel = PathBuf::from(line[3..].trim());
            file_statuses.insert(rel, GitFileStatus::Ignored);
            continue;
        }

        let x = line.as_bytes()[0] as char;
        let y = line.as_bytes()[1] as char;
        let status = classify_file_status(x, y);

        if let Some(status) = status {
            let rel = parse_path_from_status_line(line);
            if !rel.as_os_str().is_empty() {
                file_statuses.insert(rel, status);
            }
        }
    }

    Some(GitRepoStatus {
        branch,
        repo_root,
        file_statuses,
    })
}

fn parse_path_from_status_line(line: &str) -> PathBuf {
    let payload = line.get(3..).unwrap_or("").trim();
    let path = if let Some((_, to_path)) = payload.rsplit_once(" -> ") {
        to_path.trim()
    } else {
        payload
    };
    PathBuf::from(path)
}

fn classify_file_status(x: char, y: char) -> Option<GitFileStatus> {
    if is_conflict_state(x, y) {
        return Some(GitFileStatus::Conflicted);
    }
    if x == 'R' || y == 'R' {
        return Some(GitFileStatus::Renamed);
    }
    if x == 'D' || y == 'D' {
        return Some(GitFileStatus::Deleted);
    }
    if x == 'A' || y == 'A' {
        return Some(GitFileStatus::Added);
    }
    if x == 'C' || y == 'C' {
        return Some(GitFileStatus::Copied);
    }
    if matches!(x, 'M' | 'T') || matches!(y, 'M' | 'T') {
        return Some(GitFileStatus::Modified);
    }
    None
}

fn parse_branch_name(head_line: &str) -> Option<String> {
    let mut detail = head_line.trim_start_matches("## ").trim();
    if detail.is_empty() {
        return None;
    }
    if let Some(branch) = detail.strip_prefix("No commits yet on ") {
        let branch = branch.trim();
        return if branch.is_empty() {
            None
        } else {
            Some(branch.to_string())
        };
    }
    if detail == "HEAD (no branch)" {
        return Some("detached".to_string());
    }
    if let Some((left, _)) = detail.split_once("...") {
        detail = left.trim();
    } else if let Some((left, _)) = detail.split_once(' ') {
        detail = left.trim();
    }
    if detail.is_empty() {
        None
    } else {
        Some(detail.to_string())
    }
}

fn is_conflict_state(x: char, y: char) -> bool {
    matches!((x, y), ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D'))
}

#[cfg(test)]
mod tests {
    use super::{parse_porcelain_status, GitFileStatus};
    use std::path::{Path, PathBuf};

    #[test]
    fn parses_branch_and_per_file_statuses() {
        let input = "## main...origin/main [ahead 1]\n M src/lib.rs\nD  old.txt\nR  old-name.txt -> new-name.txt\nA  added.txt\nUU conflict.txt\n?? tests/new_test.rs\n!! target/\n";
        let parsed = parse_porcelain_status(input, PathBuf::from("/tmp/repo")).expect("parsed");

        assert_eq!(parsed.branch, "main");
        assert_eq!(parsed.repo_root, PathBuf::from("/tmp/repo"));
        assert_eq!(
            parsed.file_statuses.get(Path::new("src/lib.rs")),
            Some(&GitFileStatus::Modified)
        );
        assert_eq!(
            parsed.file_statuses.get(Path::new("old.txt")),
            Some(&GitFileStatus::Deleted)
        );
        assert_eq!(
            parsed.file_statuses.get(Path::new("new-name.txt")),
            Some(&GitFileStatus::Renamed)
        );
        assert_eq!(
            parsed.file_statuses.get(Path::new("added.txt")),
            Some(&GitFileStatus::Added)
        );
        assert_eq!(
            parsed.file_statuses.get(Path::new("conflict.txt")),
            Some(&GitFileStatus::Conflicted)
        );
        assert_eq!(
            parsed.file_statuses.get(Path::new("tests/new_test.rs")),
            Some(&GitFileStatus::Untracked)
        );
        assert_eq!(
            parsed.file_statuses.get(Path::new("target/")),
            Some(&GitFileStatus::Ignored)
        );
        assert_eq!(parsed.change_count(), 6);
    }

    #[test]
    fn labels_match_expected_short_codes() {
        assert_eq!(GitFileStatus::Added.label(), "A");
        assert_eq!(GitFileStatus::Modified.label(), "M");
        assert_eq!(GitFileStatus::Deleted.label(), "D");
        assert_eq!(GitFileStatus::Renamed.label(), "R");
        assert_eq!(GitFileStatus::Copied.label(), "C");
        assert_eq!(GitFileStatus::Untracked.label(), "?");
        assert_eq!(GitFileStatus::Conflicted.label(), "U");
        assert_eq!(GitFileStatus::Ignored.label(), "!");
    }

    #[test]
    fn parses_no_commits_yet_branch_name() {
        let input = "## No commits yet on main\n?? readme.md\n";
        let parsed = parse_porcelain_status(input, PathBuf::from("/tmp/repo")).expect("parsed");
        assert_eq!(parsed.branch, "main");
    }
}
