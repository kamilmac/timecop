use anyhow::{Context, Result};
use git2::{DiffOptions, Repository, StatusOptions};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::types::*;

/// Git client using libgit2 for native performance
pub struct GitClient {
    repo: Repository,
    path: PathBuf,
    base_branch: Option<String>,
    cached_merge_base: Option<git2::Oid>,
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
            cached_merge_base: None,
        };
        client.base_branch = client.detect_base_branch();
        client.fetch_base_branch();
        Ok(client)
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

    /// Set the base branch explicitly (e.g. from GitHub PR metadata)
    /// Fetches the remote ref to ensure it's up to date
    pub fn set_base_branch(&mut self, branch: &str) {
        let remote_name = format!("origin/{}", branch);
        if self.repo.find_reference(&format!("refs/remotes/{}", remote_name)).is_ok() {
            self.base_branch = Some(remote_name);
        } else {
            self.base_branch = Some(branch.to_string());
        }
        self.cached_merge_base = None;
        self.fetch_base_branch();
    }

    /// Re-detect the base branch (e.g. after switching branches)
    pub fn refresh_base_branch(&mut self) {
        self.base_branch = self.detect_base_branch();
        self.cached_merge_base = None;
        self.fetch_base_branch();
    }

    /// Compute and cache the merge base OID so downstream calls don't repeat the graph walk
    pub fn refresh_merge_base_cache(&mut self) {
        let base = match &self.base_branch {
            Some(b) => b.clone(),
            None => return,
        };
        let oid = (|| -> Result<git2::Oid> {
            let base_commit = self.resolve_commit(&base)?;
            let head_commit = self.repo.head()?.peel_to_commit()?;
            self.repo
                .merge_base(head_commit.id(), base_commit.id())
                .context("Failed to find merge-base")
        })();
        self.cached_merge_base = oid.ok();
    }

    /// Fetch the base branch from origin so diffs match GitHub's view
    pub fn fetch_base_branch(&self) {
        let base = match &self.base_branch {
            Some(b) if b.starts_with("origin/") => b.strip_prefix("origin/").unwrap(),
            _ => return,
        };
        let mut remote = match self.repo.find_remote("origin") {
            Ok(r) => r,
            Err(_) => return,
        };
        let _ = remote.fetch(&[base], None, None);
    }

    /// Detect the base branch (main, master, etc.)
    /// Prefers origin/main over local main to match GitHub's behavior
    fn detect_base_branch(&self) -> Option<String> {
        // Prefer remote branches (matches GitHub PR behavior)
        for name in &["main", "master"] {
            let remote_name = format!("origin/{}", name);
            if self.repo.find_reference(&format!("refs/remotes/{}", remote_name)).is_ok() {
                return Some(remote_name);
            }
        }
        // Fall back to local branches
        for name in &["main", "master"] {
            if self.repo.find_branch(name, git2::BranchType::Local).is_ok() {
                return Some(name.to_string());
            }
        }
        None
    }

    /// Get combined status: branch changes + uncommitted, with uncommitted flag
    pub fn status(&self) -> Result<Vec<StatusEntry>> {
        // Get uncommitted files (working tree + index)
        let uncommitted_paths = self.get_uncommitted_paths()?;

        // Get branch changes (committed vs base)
        let base = match &self.base_branch {
            Some(b) => b,
            None => {
                // No base branch - just show uncommitted
                return self.uncommitted_status();
            }
        };

        let merge_base = self.merge_base_commit(base)?;
        let head_commit = self.repo.head()?.peel_to_commit()?;

        let base_tree = merge_base.tree()?;
        let head_tree = head_commit.tree()?;

        let diff = self.repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;

        let mut entries = Vec::new();
        let mut seen_paths = HashSet::new();

        // Add committed changes
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

            let uncommitted = uncommitted_paths.contains(&path);
            entries.push(StatusEntry { path: path.clone(), status, uncommitted, entry_type: EntryType::Tracked });
            seen_paths.insert(path);
        }

        // Add uncommitted-only files (not in branch diff)
        for path in &uncommitted_paths {
            if !seen_paths.contains(path) {
                // Determine status from working tree
                let status = self.get_file_status(path)?;
                entries.push(StatusEntry {
                    path: path.clone(),
                    status,
                    uncommitted: true,
                    entry_type: EntryType::Tracked,
                });
            }
        }

        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    /// Create StatusOptions configured for tracking uncommitted files
    fn status_opts() -> StatusOptions {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true);
        opts
    }

    /// Get paths of uncommitted files
    fn get_uncommitted_paths(&self) -> Result<HashSet<String>> {
        let mut opts = Self::status_opts();
        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut paths = HashSet::new();

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                paths.insert(path.to_string());
            }
        }

        Ok(paths)
    }

    /// Get status for uncommitted-only files
    fn uncommitted_status(&self) -> Result<Vec<StatusEntry>> {
        let mut opts = Self::status_opts();
        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut entries = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let git_status = entry.status();

            let status = if git_status.is_index_new() || git_status.is_wt_new() {
                FileStatus::Added
            } else if git_status.is_index_modified() || git_status.is_wt_modified() {
                FileStatus::Modified
            } else if git_status.is_index_deleted() || git_status.is_wt_deleted() {
                FileStatus::Deleted
            } else if git_status.is_index_renamed() || git_status.is_wt_renamed() {
                FileStatus::Renamed
            } else {
                continue;
            };

            entries.push(StatusEntry {
                path,
                status,
                uncommitted: true,
                entry_type: EntryType::Tracked,
            });
        }

        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    /// Get file status from working tree
    fn get_file_status(&self, path: &str) -> Result<FileStatus> {
        let mut opts = StatusOptions::new();
        opts.pathspec(path);

        let statuses = self.repo.statuses(Some(&mut opts))?;

        for entry in statuses.iter() {
            let git_status = entry.status();
            if git_status.is_index_new() || git_status.is_wt_new() {
                return Ok(FileStatus::Added);
            } else if git_status.is_index_modified() || git_status.is_wt_modified() {
                return Ok(FileStatus::Modified);
            } else if git_status.is_index_deleted() || git_status.is_wt_deleted() {
                return Ok(FileStatus::Deleted);
            } else if git_status.is_index_renamed() || git_status.is_wt_renamed() {
                return Ok(FileStatus::Renamed);
            }
        }

        Ok(FileStatus::Modified)
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

    /// Get diff statistics for a specific timeline position
    pub fn diff_stats_at_position(&self, position: super::TimelinePosition) -> Result<DiffStats> {
        use super::TimelinePosition;

        // Browse mode has no diff stats
        if matches!(position, TimelinePosition::Browse) {
            return Ok(DiffStats::default());
        }

        let diff = match position {
            TimelinePosition::Browse => unreachable!(), // Handled above
            TimelinePosition::FullDiff => {
                let base = match &self.base_branch {
                    Some(b) => b,
                    None => return Ok(DiffStats::default()),
                };
                let merge_base = self.merge_base_commit(base)?;
                let base_tree = merge_base.tree()?;
                self.repo.diff_tree_to_workdir_with_index(Some(&base_tree), None)?
            }
            TimelinePosition::Wip => {
                let head_tree = self.repo.head()?.peel_to_tree()?;
                self.repo.diff_tree_to_workdir(Some(&head_tree), None)?
            }
            TimelinePosition::CommitDiff(n) => {
                let old_commit = self.commit_at_offset(n)?;
                let new_commit = self.commit_at_offset(n - 1)?;
                let old_tree = old_commit.tree()?;
                let new_tree = new_commit.tree()?;
                self.repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?
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

    /// Find the merge-base (common ancestor) between HEAD and base branch.
    /// Uses cached OID when available to avoid repeated graph walks.
    fn merge_base_commit(&self, base: &str) -> Result<git2::Commit<'_>> {
        let merge_base_oid = if let Some(oid) = self.cached_merge_base {
            oid
        } else {
            let base_commit = self.resolve_commit(base)?;
            let head_commit = self.repo.head()?.peel_to_commit()?;
            self.repo
                .merge_base(head_commit.id(), base_commit.id())
                .context("Failed to find merge-base")?
        };

        self.repo
            .find_commit(merge_base_oid)
            .context("Failed to find merge-base commit")
    }

    /// Get the repository path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if there are any uncommitted changes (staged or unstaged)
    pub fn has_uncommitted_changes(&self) -> bool {
        self.get_uncommitted_paths()
            .map(|paths| !paths.is_empty())
            .unwrap_or(false)
    }

    /// Count commits since base branch (for timeline)
    /// Uses first-parent traversal to match GitHub's PR behavior
    pub fn commit_count_since_base(&self) -> Result<usize> {
        let base = match &self.base_branch {
            Some(b) => b,
            None => return Ok(0),
        };

        let merge_base = self.merge_base_commit(base)?;
        let head_commit = self.repo.head()?.peel_to_commit()?;

        let mut count = 0;
        let mut revwalk = self.repo.revwalk()?;
        revwalk.simplify_first_parent()?;
        revwalk.push(head_commit.id())?;
        revwalk.hide(merge_base.id())?;

        for _ in revwalk {
            count += 1;
        }

        Ok(count)
    }

    /// Get commit at HEAD~n (first-parent only, matches GitHub PR behavior)
    fn commit_at_offset(&self, offset: usize) -> Result<git2::Commit<'_>> {
        let head = self.repo.head()?.peel_to_commit()?;
        if offset == 0 {
            return Ok(head);
        }

        // Walk first-parent only to match GitHub's commit ordering
        let mut revwalk = self.repo.revwalk()?;
        revwalk.simplify_first_parent()?;
        revwalk.push(head.id())?;

        let mut current = head;
        for (i, oid) in revwalk.enumerate() {
            if i == offset {
                return self.repo.find_commit(oid?).context("Failed to find commit");
            }
            if i > offset {
                break;
            }
            current = self.repo.find_commit(oid?)?;
        }

        // If we didn't find enough commits, return the last one
        Ok(current)
    }

    /// Get commit summary (first line of message) at HEAD~n
    pub fn commit_summary_at_offset(&self, offset: usize) -> Result<String> {
        let commit = self.commit_at_offset(offset)?;
        let summary = commit.summary().unwrap_or("(no message)");
        Ok(summary.to_string())
    }

    /// Get diff for a file at a specific timeline position
    pub fn diff_at_position(&self, path: &str, position: super::TimelinePosition) -> Result<String> {
        use super::TimelinePosition;

        // Browse mode: return file content, not diff
        if matches!(position, TimelinePosition::Browse) {
            return self.read_file(path);
        }

        let base = match &self.base_branch {
            Some(b) => b,
            None => return self.working_diff(path),
        };

        let merge_base = self.merge_base_commit(base)?;
        let base_tree = merge_base.tree()?;

        let mut opts = DiffOptions::new();
        opts.pathspec(path);

        match position {
            TimelinePosition::Browse => unreachable!(), // Handled above
            TimelinePosition::FullDiff => {
                // Base to working tree (all changes: committed + uncommitted)
                let diff = self.repo.diff_tree_to_workdir_with_index(Some(&base_tree), Some(&mut opts))?;
                let result = self.diff_to_string(&diff)?;
                if result.is_empty() {
                    return self.format_new_file(path);
                }
                Ok(result)
            }
            TimelinePosition::Wip => {
                // HEAD to working tree (uncommitted only)
                let head_tree = self.repo.head()?.peel_to_tree()?;
                let diff = self.repo.diff_tree_to_workdir(Some(&head_tree), Some(&mut opts))?;
                let result = self.diff_to_string(&diff)?;
                if result.is_empty() {
                    return self.format_new_file(path);
                }
                Ok(result)
            }
            TimelinePosition::CommitDiff(n) => {
                // Single commit: HEAD~n → HEAD~(n-1)
                let old_commit = self.commit_at_offset(n)?;
                let new_commit = self.commit_at_offset(n - 1)?;
                let old_tree = old_commit.tree()?;
                let new_tree = new_commit.tree()?;
                let diff = self.repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), Some(&mut opts))?;
                let result = self.diff_to_string(&diff)?;
                Ok(result)
            }
        }
    }

    /// Get file status at a specific timeline position
    pub fn status_at_position(&self, position: super::TimelinePosition) -> Result<Vec<StatusEntry>> {
        use super::TimelinePosition;

        log::debug!("status_at_position: {:?}", position);

        match position {
            TimelinePosition::Browse => {
                // Show all tracked files in the repository
                self.list_all_files()
            }
            TimelinePosition::FullDiff => {
                // Show all committed changes: base → HEAD
                self.status()
            }
            TimelinePosition::Wip => {
                // Show ONLY uncommitted changes: HEAD → working tree
                self.uncommitted_status()
            }
            TimelinePosition::CommitDiff(n) => {
                // Show changes from single commit: HEAD~n → HEAD~(n-1)
                log::debug!("Getting single commit diff: HEAD~{} → HEAD~{}", n, n - 1);

                let old_commit = self.commit_at_offset(n)?;
                let new_commit = self.commit_at_offset(n - 1)?;

                let old_tree = old_commit.tree()?;
                let new_tree = new_commit.tree()?;

                let diff = self.repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;

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

                    entries.push(StatusEntry {
                        path,
                        status,
                        uncommitted: false,
                        entry_type: EntryType::Tracked,
                    });
                }

                entries.sort_by(|a, b| a.path.cmp(&b.path));
                Ok(entries)
            }
        }
    }

    /// List all files in the repository directory (for browse/files mode)
    /// Walks filesystem and marks gitignored files
    fn list_all_files(&self) -> Result<Vec<StatusEntry>> {
        let mut entries = Vec::new();
        self.walk_dir(&self.path, &mut entries, 0)?;
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    /// Walk an ignored directory and return its entries (for lazy expansion)
    pub fn list_ignored_dir(&self, rel_dir: &str) -> Result<Vec<StatusEntry>> {
        let abs_dir = self.path.join(rel_dir);
        let mut entries = Vec::new();
        self.walk_dir(&abs_dir, &mut entries, 0)?;
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    /// Recursively walk directory and collect files
    /// Max depth of 20 to prevent runaway recursion
    fn walk_dir(&self, dir: &Path, entries: &mut Vec<StatusEntry>, depth: usize) -> Result<()> {
        const MAX_DEPTH: usize = 20;
        if depth > MAX_DEPTH {
            return Ok(());
        }
        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return Ok(()), // Skip unreadable directories
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Skip .git directory
            if file_name_str == ".git" {
                continue;
            }

            // Get relative path from repo root
            let rel_path = path.strip_prefix(&self.path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            if rel_path.is_empty() {
                continue;
            }

            // Check if path is ignored
            let ignored = self.repo.status_should_ignore(Path::new(&rel_path)).unwrap_or(false);

            if path.is_dir() {
                if ignored {
                    // Add ignored directory directly (don't recurse into it)
                    entries.push(StatusEntry {
                        path: rel_path,
                        status: FileStatus::Unchanged,
                        uncommitted: false,
                        entry_type: EntryType::IgnoredDir,
                    });
                } else {
                    self.walk_dir(&path, entries, depth + 1)?;
                }
            } else {
                let entry_type = if ignored { EntryType::Ignored } else { EntryType::Tracked };
                entries.push(StatusEntry {
                    path: rel_path,
                    status: FileStatus::Unchanged,
                    uncommitted: false,
                    entry_type,
                });
            }
        }

        Ok(())
    }
}
