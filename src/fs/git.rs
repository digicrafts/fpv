use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};

use git2::{DiffFormat, DiffOptions, Repository, Status, StatusOptions};

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

#[derive(Debug, Clone)]
pub struct GitStatusUpdate {
    pub requested_path: PathBuf,
    pub status: Option<GitRepoStatus>,
}

pub struct GitStatusWorker {
    request_tx: Option<Sender<PathBuf>>,
    update_rx: Receiver<GitStatusUpdate>,
    join_handle: Option<JoinHandle<()>>,
}

impl GitStatusWorker {
    pub fn spawn() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<PathBuf>();
        let (update_tx, update_rx) = mpsc::channel::<GitStatusUpdate>();

        let join_handle = thread::spawn(move || {
            while let Ok(mut path) = request_rx.recv() {
                while let Ok(next_path) = request_rx.try_recv() {
                    path = next_path;
                }

                let status = git_repo_status_for_path(&path);
                if update_tx
                    .send(GitStatusUpdate {
                        requested_path: path,
                        status,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            request_tx: Some(request_tx),
            update_rx,
            join_handle: Some(join_handle),
        }
    }

    pub fn request(&self, path: PathBuf) -> bool {
        self.request_tx
            .as_ref()
            .is_some_and(|tx| tx.send(path).is_ok())
    }

    pub fn latest_update(&self) -> Option<GitStatusUpdate> {
        let mut latest = None;

        loop {
            match self.update_rx.try_recv() {
                Ok(update) => latest = Some(update),
                Err(TryRecvError::Empty) => return latest,
                Err(TryRecvError::Disconnected) => return latest,
            }
        }
    }
}

impl Drop for GitStatusWorker {
    fn drop(&mut self) {
        self.request_tx.take();
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

fn open_repo(path: &Path) -> Option<Repository> {
    Repository::discover(path).ok()
}

fn repo_root(repo: &Repository) -> Option<PathBuf> {
    repo.workdir().map(|p| p.to_path_buf())
}

fn head_branch_name(repo: &Repository) -> String {
    if repo.head_detached().unwrap_or(false) {
        return "detached".to_string();
    }
    match repo.head() {
        Ok(reference) => reference.shorthand().unwrap_or("HEAD").to_string(),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
            // No commits yet — try to extract branch name from the symbolic HEAD
            repo.find_reference("HEAD")
                .ok()
                .and_then(|r| r.symbolic_target().map(String::from))
                .and_then(|target| target.strip_prefix("refs/heads/").map(String::from))
                .unwrap_or_else(|| "HEAD".to_string())
        }
        Err(_) => "HEAD".to_string(),
    }
}

fn classify_status(status: Status) -> Option<GitFileStatus> {
    if status.is_conflicted() {
        return Some(GitFileStatus::Conflicted);
    }
    if status.is_ignored() {
        return Some(GitFileStatus::Ignored);
    }
    if status.contains(Status::WT_NEW) || status.contains(Status::INDEX_NEW) {
        // INDEX_NEW = staged new file (Added), WT_NEW = untracked
        if status.contains(Status::WT_NEW) && !status.contains(Status::INDEX_NEW) {
            return Some(GitFileStatus::Untracked);
        }
        return Some(GitFileStatus::Added);
    }
    if status.contains(Status::INDEX_RENAMED) || status.contains(Status::WT_RENAMED) {
        return Some(GitFileStatus::Renamed);
    }
    if status.contains(Status::INDEX_DELETED) || status.contains(Status::WT_DELETED) {
        return Some(GitFileStatus::Deleted);
    }
    if status.contains(Status::INDEX_TYPECHANGE) || status.contains(Status::WT_TYPECHANGE) {
        return Some(GitFileStatus::Modified);
    }
    if status.contains(Status::INDEX_MODIFIED) || status.contains(Status::WT_MODIFIED) {
        return Some(GitFileStatus::Modified);
    }
    None
}

pub fn git_repo_status_for_path(path: &Path) -> Option<GitRepoStatus> {
    let repo = open_repo(path)?;
    let root = repo_root(&repo)?;
    let branch = head_branch_name(&repo);

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(false)
        .include_ignored(true)
        .recurse_ignored_dirs(false);

    let statuses = repo.statuses(Some(&mut opts)).ok()?;
    let mut file_statuses = HashMap::new();

    for entry in statuses.iter() {
        let Some(entry_path) = entry.path() else {
            continue;
        };
        let git_status = classify_status(entry.status());
        if let Some(status) = git_status {
            // For renames, use the new path if available
            let path_str = entry
                .head_to_index()
                .and_then(|d| d.new_file().path().map(|p| p.to_path_buf()))
                .or_else(|| {
                    entry
                        .index_to_workdir()
                        .and_then(|d| d.new_file().path().map(|p| p.to_path_buf()))
                })
                .unwrap_or_else(|| PathBuf::from(entry_path));
            file_statuses.insert(path_str, status);
        }
    }

    Some(GitRepoStatus {
        branch,
        repo_root: root,
        file_statuses,
    })
}

fn normalized_file_path(file_path: &Path) -> PathBuf {
    let absolute_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        std::env::current_dir()
            .ok()
            .map(|cwd| cwd.join(file_path))
            .unwrap_or_else(|| file_path.to_path_buf())
    };
    fs::canonicalize(&absolute_path).unwrap_or(absolute_path)
}

pub fn git_diff_for_file(file_path: &Path) -> Option<String> {
    let file_path = normalized_file_path(file_path);
    let repo = open_repo(file_path.parent().unwrap_or(&file_path))?;
    let root = repo_root(&repo)?;
    let rel_path = file_path.strip_prefix(&root).ok()?;
    let rel_str = rel_path.to_str()?;

    // Try diff HEAD to working directory for this specific file
    let head_tree = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .and_then(|c| c.tree().ok());

    let mut opts = DiffOptions::new();
    opts.pathspec(rel_str);

    // diff HEAD tree against workdir (includes both staged and unstaged changes)
    let diff = if let Some(ref tree) = head_tree {
        repo.diff_tree_to_workdir_with_index(Some(tree), Some(&mut opts))
            .ok()?
    } else {
        // No HEAD (empty repo) — diff index to workdir
        repo.diff_index_to_workdir(None, Some(&mut opts)).ok()?
    };

    if diff.deltas().len() == 0 {
        // No changes found via tree diff; for untracked files, generate a synthetic diff
        // by reading the file content directly
        let workdir_content = fs::read_to_string(&file_path).ok()?;
        if workdir_content.trim().is_empty() {
            return None;
        }
        // Build a diff-like output for untracked files
        let mut output = format!(
            "diff --git a/{rel} b/{rel}\nnew file mode 100644\n--- /dev/null\n+++ b/{rel}\n",
            rel = rel_str
        );
        let lines: Vec<&str> = workdir_content.lines().collect();
        output.push_str(&format!("@@ -0,0 +1,{} @@\n", lines.len()));
        for line in &lines {
            output.push('+');
            output.push_str(line);
            output.push('\n');
        }
        return Some(output);
    }

    // Format as unified diff text
    let mut diff_text = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        match line.origin() {
            '+' | '-' | ' ' => {
                diff_text.push(line.origin());
            }
            'H' | 'F' => {
                // Header and file header lines — include as-is
            }
            _ => {}
        }
        if let Ok(content) = std::str::from_utf8(line.content()) {
            diff_text.push_str(content);
        }
        true
    })
    .ok()?;

    if diff_text.trim().is_empty() {
        None
    } else {
        Some(diff_text)
    }
}

pub fn git_head_content_for_file(file_path: &Path) -> Option<String> {
    let file_path = normalized_file_path(file_path);
    let repo = open_repo(file_path.parent().unwrap_or(&file_path))?;
    let root = repo_root(&repo)?;
    let rel_path = file_path.strip_prefix(&root).ok()?;

    let head_commit = repo.head().ok()?.peel_to_commit().ok()?;
    let tree = head_commit.tree().ok()?;

    match tree.get_path(rel_path) {
        Ok(entry) => {
            let object = entry.to_object(&repo).ok()?;
            let blob = object.as_blob()?;
            // Return content as UTF-8 string (lossy for binary)
            Some(String::from_utf8_lossy(blob.content()).into_owned())
        }
        Err(_) => {
            // File does not exist in HEAD (new/untracked file)
            Some(String::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GitFileStatus;

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
}
