use anyhow::{Context, Result};
use git2::{DiffOptions, Repository, StatusOptions};
use std::path::{Path, PathBuf};

use super::types::*;

/// Git client using libgit2 for native performance
pub struct GitClient {
    repo: Repository,
    path: PathBuf,
    base_branch: Option<String>,
}

impl GitClient {
    /// Open a git repository at the given path
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let repo = Repository::open(&path).context("Failed to open git repository")?;

        let mut client = Self {
            repo,
            path,
            base_branch: None,
        };
        client.base_branch = client.detect_base_branch();
        Ok(client)
    }

    /// Check if this is a valid git repository
    pub fn is_repo(&self) -> bool {
        !self.repo.is_bare()
    }

    /// Get the current branch name
    pub fn current_branch(&self) -> Result<String> {
        let head = self.repo.head().context("Failed to get HEAD")?;
        let branch = head
            .shorthand()
            .unwrap_or("HEAD")
            .to_string();
        Ok(branch)
    }

    /// Get the base branch name
    pub fn base_branch(&self) -> Option<&str> {
        self.base_branch.as_deref()
    }

    /// Detect the base branch (main, master, etc.)
    fn detect_base_branch(&self) -> Option<String> {
        // Try common branch names
        for name in &["main", "master"] {
            if self.repo.find_branch(name, git2::BranchType::Local).is_ok() {
                return Some(name.to_string());
            }
            // Try remote
            let remote_name = format!("origin/{}", name);
            if self.repo.find_reference(&format!("refs/remotes/{}", remote_name)).is_ok() {
                return Some(remote_name);
            }
        }
        None
    }

    /// Get status of changed files
    pub fn status(&self, mode: DiffMode) -> Result<Vec<StatusEntry>> {
        match mode {
            DiffMode::Working => self.working_status(),
            DiffMode::Branch => self.branch_status(),
        }
    }

    /// Get uncommitted changes (working tree status)
    fn working_status(&self) -> Result<Vec<StatusEntry>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut entries = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = entry.status();

            let file_status = if status.is_index_new() || status.is_wt_new() {
                FileStatus::Added
            } else if status.is_index_modified() || status.is_wt_modified() {
                FileStatus::Modified
            } else if status.is_index_deleted() || status.is_wt_deleted() {
                FileStatus::Deleted
            } else if status.is_index_renamed() || status.is_wt_renamed() {
                FileStatus::Renamed
            } else {
                continue;
            };

            entries.push(StatusEntry {
                path,
                status: file_status,
            });
        }

        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    /// Get all changes compared to base branch
    fn branch_status(&self) -> Result<Vec<StatusEntry>> {
        let base = match &self.base_branch {
            Some(b) => b,
            None => return self.working_status(),
        };

        let base_commit = self.resolve_commit(base)?;
        let head_commit = self.repo.head()?.peel_to_commit()?;

        let base_tree = base_commit.tree()?;
        let head_tree = head_commit.tree()?;

        let diff = self.repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;

        let mut entries = Vec::new();
        for delta in diff.deltas() {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let status = match delta.status() {
                git2::Delta::Added => FileStatus::Added,
                git2::Delta::Deleted => FileStatus::Deleted,
                git2::Delta::Modified => FileStatus::Modified,
                git2::Delta::Renamed => FileStatus::Renamed,
                _ => continue,
            };

            entries.push(StatusEntry { path, status });
        }

        // Also include working tree changes
        let working = self.working_status()?;
        for entry in working {
            if !entries.iter().any(|e| e.path == entry.path) {
                entries.push(entry);
            }
        }

        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    /// List all tracked files
    pub fn list_all_files(&self) -> Result<Vec<StatusEntry>> {
        let head = self.repo.head()?.peel_to_tree()?;
        let mut entries = Vec::new();

        head.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if entry.kind() == Some(git2::ObjectType::Blob) {
                let path = if dir.is_empty() {
                    entry.name().unwrap_or("").to_string()
                } else {
                    format!("{}{}", dir, entry.name().unwrap_or(""))
                };
                entries.push(StatusEntry {
                    path,
                    status: FileStatus::Unchanged,
                });
            }
            git2::TreeWalkResult::Ok
        })?;

        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    /// List markdown files only
    pub fn list_doc_files(&self) -> Result<Vec<StatusEntry>> {
        let all = self.list_all_files()?;
        Ok(all
            .into_iter()
            .filter(|e| e.path.ends_with(".md") || e.path.ends_with(".markdown"))
            .collect())
    }

    /// Get diff for a specific file
    pub fn diff(&self, path: &str, mode: DiffMode) -> Result<String> {
        match mode {
            DiffMode::Working => self.working_diff(path),
            DiffMode::Branch => self.branch_diff(path),
        }
    }

    fn working_diff(&self, path: &str) -> Result<String> {
        let mut opts = DiffOptions::new();
        opts.pathspec(path);

        let diff = self.repo.diff_index_to_workdir(None, Some(&mut opts))?;
        let result = self.diff_to_string(&diff)?;

        // If no diff output, file might be untracked - show as new file
        if result.is_empty() {
            return self.format_new_file(path);
        }
        Ok(result)
    }

    fn format_new_file(&self, path: &str) -> Result<String> {
        let content = self.read_file(path)?;
        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len();

        let mut result = format!("diff --git a/{} b/{}\n", path, path);
        result.push_str("new file\n");
        result.push_str(&format!("@@ -0,0 +1,{} @@\n", line_count));

        for line in lines {
            result.push('+');
            result.push_str(line);
            result.push('\n');
        }

        Ok(result)
    }

    fn branch_diff(&self, path: &str) -> Result<String> {
        let base = match &self.base_branch {
            Some(b) => b,
            None => return self.working_diff(path),
        };

        let base_commit = self.resolve_commit(base)?;
        let base_tree = base_commit.tree()?;

        let mut opts = DiffOptions::new();
        opts.pathspec(path);

        let diff = self.repo.diff_tree_to_workdir(Some(&base_tree), Some(&mut opts))?;
        let result = self.diff_to_string(&diff)?;

        // If no diff output, file might be new - show as new file
        if result.is_empty() {
            return self.format_new_file(path);
        }
        Ok(result)
    }

    /// Get combined diff for multiple files
    pub fn diff_files(&self, paths: &[String], mode: DiffMode) -> Result<String> {
        let mut result = String::new();
        for path in paths {
            let diff = self.diff(path, mode)?;
            if !diff.is_empty() {
                result.push_str(&diff);
                result.push('\n');
            }
        }
        Ok(result)
    }

    fn diff_to_string(&self, diff: &git2::Diff) -> Result<String> {
        let mut result = String::new();
        diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
            match line.origin() {
                'F' => {
                    // File header
                    if let (Some(old), Some(new)) = (delta.old_file().path(), delta.new_file().path()) {
                        result.push_str(&format!("diff --git a/{} b/{}\n",
                            old.display(), new.display()));
                    }
                }
                'H' => {
                    // Hunk header
                    if let Some(h) = hunk {
                        result.push_str(&format!("@@ -{},{} +{},{} @@\n",
                            h.old_start(), h.old_lines(),
                            h.new_start(), h.new_lines()));
                    }
                }
                '+' | '-' | ' ' => {
                    result.push(line.origin());
                    if let Ok(content) = std::str::from_utf8(line.content()) {
                        result.push_str(content);
                    }
                }
                _ => {
                    if let Ok(content) = std::str::from_utf8(line.content()) {
                        result.push_str(content);
                    }
                }
            }
            true
        })?;
        Ok(result)
    }

    /// Read file content
    pub fn read_file(&self, path: &str) -> Result<String> {
        let full_path = self.path.join(path);
        std::fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read file: {}", path))
    }

    /// Get recent commits
    pub fn log(&self, max: usize) -> Result<Vec<Commit>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;

        let mut commits = Vec::new();
        for oid in revwalk.take(max) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            let author = commit.author();
            let time = commit.time();
            let datetime = chrono::DateTime::from_timestamp(time.seconds(), 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();

            // Get commit body (message without subject line)
            let body = commit.message()
                .unwrap_or("")
                .lines()
                .skip(1)
                .skip_while(|l| l.is_empty())
                .collect::<Vec<_>>()
                .join("\n");

            // Get diff stats (compare with parent)
            let (files_changed, insertions, deletions) = self.commit_stats(&commit)
                .unwrap_or((0, 0, 0));

            commits.push(Commit {
                hash: oid.to_string(),
                short_hash: oid.to_string()[..7].to_string(),
                author: author.name().unwrap_or("Unknown").to_string(),
                date: datetime,
                subject: commit.summary().unwrap_or("").to_string(),
                body,
                files_changed,
                insertions,
                deletions,
            });
        }

        Ok(commits)
    }

    /// Get diff stats for a commit (files changed, insertions, deletions)
    fn commit_stats(&self, commit: &git2::Commit) -> Result<(usize, usize, usize)> {
        let tree = commit.tree()?;
        let parent_tree = commit.parent(0)
            .ok()
            .and_then(|p| p.tree().ok());

        let diff = self.repo.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&tree),
            None,
        )?;

        let stats = diff.stats()?;
        Ok((
            stats.files_changed(),
            stats.insertions(),
            stats.deletions(),
        ))
    }

    /// Get diff statistics
    pub fn diff_stats(&self, mode: DiffMode) -> Result<DiffStats> {
        let diff = match mode {
            DiffMode::Working => {
                self.repo.diff_index_to_workdir(None, None)?
            }
            DiffMode::Branch => {
                let base = match &self.base_branch {
                    Some(b) => b,
                    None => return Ok(DiffStats::default()),
                };
                let base_commit = self.resolve_commit(base)?;
                let base_tree = base_commit.tree()?;
                self.repo.diff_tree_to_workdir(Some(&base_tree), None)?
            }
        };

        let stats = diff.stats()?;
        Ok(DiffStats {
            added: stats.insertions(),
            removed: stats.deletions(),
        })
    }

    fn resolve_commit(&self, refspec: &str) -> Result<git2::Commit<'_>> {
        let obj = self.repo.revparse_single(refspec)?;
        obj.peel_to_commit()
            .context("Failed to resolve commit")
    }

    /// Get the repository path
    pub fn path(&self) -> &Path {
        &self.path
    }
}
