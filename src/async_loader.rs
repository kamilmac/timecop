//! Async loading utilities for PR data and git analysis

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::git::{GitClient, StatusEntry};
use crate::github::{GitHubClient, PrInfo, PrSummary};

/// Manages async loading of PR data and git analysis
pub struct AsyncLoader {
    // PR list loading
    pr_list_rx: Option<Receiver<Vec<PrSummary>>>,
    pr_list_loading: bool,

    // PR details loading
    pr_detail_rx: Option<Receiver<Option<PrInfo>>>,
    pr_detail_loading: bool,
    pr_detail_number: Option<u64>,

    // Suggested files loading (co-change analysis)
    suggestions_rx: Option<Receiver<Vec<StatusEntry>>>,
    suggestions_loading: bool,
}

impl Default for AsyncLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncLoader {
    pub fn new() -> Self {
        Self {
            pr_list_rx: None,
            pr_list_loading: false,
            pr_detail_rx: None,
            pr_detail_loading: false,
            pr_detail_number: None,
            suggestions_rx: None,
            suggestions_loading: false,
        }
    }

    /// Check if PR list is currently loading
    pub fn is_pr_list_loading(&self) -> bool {
        self.pr_list_loading
    }

    /// Check if PR details are currently loading
    pub fn is_pr_detail_loading(&self) -> bool {
        self.pr_detail_loading
    }

    /// Get the PR number currently being loaded
    pub fn loading_pr_number(&self) -> Option<u64> {
        if self.pr_detail_loading {
            self.pr_detail_number
        } else {
            None
        }
    }

    /// Spawn background thread to load PR list
    pub fn load_pr_list(&mut self) {
        if self.pr_list_loading {
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.pr_list_rx = Some(rx);
        self.pr_list_loading = true;

        thread::spawn(move || {
            let mut github = GitHubClient::new();
            if github.is_available() {
                match github.list_open_prs() {
                    Ok(prs) => {
                        let _ = tx.send(prs);
                    }
                    Err(e) => {
                        log::warn!("Failed to load PR list: {}", e);
                        let _ = tx.send(vec![]);
                    }
                }
            } else {
                log::debug!("GitHub CLI not available, skipping PR list load");
                let _ = tx.send(vec![]);
            }
        });
    }

    /// Spawn background thread to load PR details
    pub fn load_pr_details(&mut self, pr_number: u64) {
        if self.pr_detail_loading && self.pr_detail_number == Some(pr_number) {
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.pr_detail_rx = Some(rx);
        self.pr_detail_loading = true;
        self.pr_detail_number = Some(pr_number);

        thread::spawn(move || {
            let mut github = GitHubClient::new();
            if github.is_available() {
                match github.get_pr_by_number(pr_number) {
                    Ok(pr) => {
                        let _ = tx.send(pr);
                    }
                    Err(e) => {
                        log::warn!("Failed to load PR #{} details: {}", pr_number, e);
                        let _ = tx.send(None);
                    }
                }
            } else {
                let _ = tx.send(None);
            }
        });
    }

    /// Poll for completed PR list loading
    pub fn poll_pr_list(&mut self) -> Option<Vec<PrSummary>> {
        let rx = self.pr_list_rx.as_ref()?;
        match rx.try_recv() {
            Ok(prs) => {
                self.pr_list_loading = false;
                self.pr_list_rx = None;
                Some(prs)
            }
            Err(TryRecvError::Disconnected) => {
                log::debug!("PR list loader disconnected");
                self.pr_list_loading = false;
                self.pr_list_rx = None;
                None
            }
            Err(TryRecvError::Empty) => None,
        }
    }

    /// Poll for completed PR details loading
    /// Returns (pr_number, Option<PrInfo>) if complete
    pub fn poll_pr_details(&mut self) -> Option<(u64, Option<PrInfo>)> {
        let rx = self.pr_detail_rx.as_ref()?;
        let pr_number = self.pr_detail_number?;

        match rx.try_recv() {
            Ok(pr) => {
                self.pr_detail_loading = false;
                self.pr_detail_rx = None;
                self.pr_detail_number = None;
                Some((pr_number, pr))
            }
            Err(TryRecvError::Disconnected) => {
                log::debug!("PR detail loader disconnected for PR #{}", pr_number);
                self.pr_detail_loading = false;
                self.pr_detail_rx = None;
                self.pr_detail_number = None;
                None
            }
            Err(TryRecvError::Empty) => None,
        }
    }

    /// Check if suggestions are currently loading
    pub fn is_suggestions_loading(&self) -> bool {
        self.suggestions_loading
    }

    /// Spawn background thread to analyze co-changes and find related files
    pub fn load_suggestions(&mut self, repo_path: PathBuf, changed_files: Vec<String>) {
        if self.suggestions_loading {
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.suggestions_rx = Some(rx);
        self.suggestions_loading = true;

        thread::spawn(move || {
            match GitClient::open(&repo_path) {
                Ok(git) => {
                    match git.find_related_files(&changed_files, 10) {
                        Ok(suggestions) => {
                            log::debug!("Found {} suggested files", suggestions.len());
                            let _ = tx.send(suggestions);
                        }
                        Err(e) => {
                            log::warn!("Failed to find related files: {}", e);
                            let _ = tx.send(vec![]);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to open repo for suggestions: {}", e);
                    let _ = tx.send(vec![]);
                }
            }
        });
    }

    /// Poll for completed suggestions loading
    pub fn poll_suggestions(&mut self) -> Option<Vec<StatusEntry>> {
        let rx = self.suggestions_rx.as_ref()?;
        match rx.try_recv() {
            Ok(suggestions) => {
                self.suggestions_loading = false;
                self.suggestions_rx = None;
                Some(suggestions)
            }
            Err(TryRecvError::Disconnected) => {
                log::debug!("Suggestions loader disconnected");
                self.suggestions_loading = false;
                self.suggestions_rx = None;
                None
            }
            Err(TryRecvError::Empty) => None,
        }
    }
}
