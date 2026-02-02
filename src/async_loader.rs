//! Async loading utilities for PR data

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::github::{GitHubClient, PrInfo, PrSummary};

/// Manages async loading of PR data
pub struct AsyncLoader {
    // PR list loading
    pr_list_rx: Option<Receiver<Vec<PrSummary>>>,
    pr_list_loading: bool,

    // PR details loading
    pr_detail_rx: Option<Receiver<Option<PrInfo>>>,
    pr_detail_loading: bool,
    pr_detail_number: Option<u64>,
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
}
