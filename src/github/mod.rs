use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

/// PR information from GitHub
#[derive(Debug, Clone, Default)]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub author: String,
    pub state: String,
    pub url: String,
    pub reviews: Vec<Review>,
    pub comments: Vec<Comment>,
    pub file_comments: HashMap<String, Vec<Comment>>,
}

#[derive(Debug, Clone)]
pub struct Review {
    pub author: String,
    pub state: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub author: String,
    pub body: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub original_line: Option<u32>,
    pub side: Option<String>, // "LEFT" or "RIGHT"
}

/// GitHub client using gh CLI
pub struct GitHubClient {
    available: Option<bool>,
}

impl GitHubClient {
    pub fn new() -> Self {
        // Lazy check - don't spawn process at startup
        Self { available: None }
    }

    pub fn is_available(&mut self) -> bool {
        if self.available.is_none() {
            let result = Command::new("gh")
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            self.available = Some(result);
        }
        self.available.unwrap_or(false)
    }

    fn get_reviews(&self, pr_number: u64) -> Result<Vec<Review>> {
        let output = Command::new("gh")
            .args([
                "pr", "view",
                &pr_number.to_string(),
                "--json", "reviews",
            ])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        #[derive(Deserialize)]
        struct ReviewsResponse {
            reviews: Vec<ReviewData>,
        }

        #[derive(Deserialize)]
        struct ReviewData {
            author: AuthorData,
            state: String,
            body: String,
        }

        #[derive(Deserialize)]
        struct AuthorData {
            login: String,
        }

        let resp: ReviewsResponse = serde_json::from_slice(&output.stdout)?;
        Ok(resp.reviews
            .into_iter()
            .filter(|r| !r.state.is_empty() || !r.body.is_empty())
            .map(|r| Review {
                author: r.author.login,
                state: r.state,
                body: r.body,
            })
            .collect())
    }

    fn get_comments(&self, pr_number: u64) -> Result<(Vec<Comment>, HashMap<String, Vec<Comment>>)> {
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{{owner}}/{{repo}}/pulls/{}/comments", pr_number),
            ])
            .output()?;

        if !output.status.success() {
            return Ok((Vec::new(), HashMap::new()));
        }

        #[derive(Deserialize)]
        struct CommentData {
            user: UserData,
            body: String,
            path: Option<String>,
            line: Option<u32>,
            original_line: Option<u32>,
            side: Option<String>,
        }

        #[derive(Deserialize)]
        struct UserData {
            login: String,
        }

        let comments: Vec<CommentData> = serde_json::from_slice(&output.stdout)
            .unwrap_or_default();

        let mut general_comments = Vec::new();
        let mut file_comments: HashMap<String, Vec<Comment>> = HashMap::new();

        for c in comments {
            let comment = Comment {
                author: c.user.login,
                body: c.body,
                path: c.path.clone(),
                line: c.line,
                original_line: c.original_line,
                side: c.side,
            };

            if let Some(path) = c.path {
                file_comments.entry(path).or_default().push(comment);
            } else {
                general_comments.push(comment);
            }
        }

        Ok((general_comments, file_comments))
    }
}

/// Summary of a PR for listing
#[derive(Debug, Clone)]
pub struct PrSummary {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub branch: String,
    pub updated_at: String,
    pub review_requested: bool, // true if current user is requested reviewer
}

impl GitHubClient {
    /// Get PR info by number
    pub fn get_pr_by_number(&mut self, pr_number: u64) -> Result<Option<PrInfo>> {
        if !self.is_available() {
            return Ok(None);
        }

        let output = Command::new("gh")
            .args([
                "pr",
                "view",
                &pr_number.to_string(),
                "--json",
                "number,title,body,author,state,url",
            ])
            .output()
            .context("Failed to run gh pr view")?;

        if !output.status.success() {
            return Ok(None);
        }

        #[derive(Deserialize)]
        struct PrBasic {
            number: u64,
            title: String,
            body: String,
            author: Author,
            state: String,
            url: String,
        }

        #[derive(Deserialize)]
        struct Author {
            login: String,
        }

        let basic: PrBasic =
            serde_json::from_slice(&output.stdout).context("Failed to parse PR JSON")?;

        let mut pr_info = PrInfo {
            number: basic.number,
            title: basic.title,
            body: basic.body,
            author: basic.author.login,
            state: basic.state,
            url: basic.url,
            ..Default::default()
        };

        // Get reviews
        if let Ok(reviews) = self.get_reviews(basic.number) {
            pr_info.reviews = reviews;
        }

        // Get comments
        if let Ok((comments, file_comments)) = self.get_comments(basic.number) {
            pr_info.comments = comments;
            pr_info.file_comments = file_comments;
        }

        Ok(Some(pr_info))
    }

    /// Get current GitHub user login
    fn get_current_user(&self) -> Option<String> {
        let output = Command::new("gh")
            .args(["api", "user", "--jq", ".login"])
            .output()
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }

    /// List open PRs for the current repo
    pub fn list_open_prs(&mut self) -> Result<Vec<PrSummary>> {
        if !self.is_available() {
            return Ok(Vec::new());
        }

        // Get current user for review request check
        let current_user = self.get_current_user();

        let output = Command::new("gh")
            .args([
                "pr", "list",
                "--state", "open",
                "--json", "number,title,author,headRefName,updatedAt,reviewRequests",
                "--limit", "50",
            ])
            .output()
            .context("Failed to run gh pr list")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        #[derive(Deserialize)]
        struct PrData {
            number: u64,
            title: String,
            author: Author,
            #[serde(rename = "headRefName")]
            head_ref_name: String,
            #[serde(rename = "updatedAt")]
            updated_at: String,
            #[serde(rename = "reviewRequests", default)]
            review_requests: Vec<ReviewRequest>,
        }

        #[derive(Deserialize)]
        struct Author {
            login: String,
        }

        #[derive(Deserialize)]
        struct ReviewRequest {
            login: Option<String>,
        }

        let prs: Vec<PrData> = serde_json::from_slice(&output.stdout)
            .unwrap_or_default();

        Ok(prs.into_iter().map(|p| {
            let review_requested = current_user.as_ref().map(|user| {
                p.review_requests.iter().any(|r| {
                    r.login.as_ref() == Some(user)
                })
            }).unwrap_or(false);

            PrSummary {
                number: p.number,
                title: p.title,
                author: p.author.login,
                branch: p.head_ref_name,
                updated_at: p.updated_at.split('T').next().unwrap_or("").to_string(),
                review_requested,
            }
        }).collect())
    }

    /// Checkout a PR branch
    pub fn checkout_pr(&self, pr_number: u64) -> Result<()> {
        Command::new("gh")
            .args(["pr", "checkout", &pr_number.to_string()])
            .output()
            .context("Failed to checkout PR")?;
        Ok(())
    }

    /// Open PR in browser
    pub fn open_pr_in_browser(&self, pr_number: u64) -> Result<()> {
        Command::new("gh")
            .args(["pr", "view", &pr_number.to_string(), "--web"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("Failed to open PR in browser")?;
        Ok(())
    }

    /// Approve a PR
    pub fn approve_pr(&self, pr_number: u64) -> Result<()> {
        let output = Command::new("gh")
            .args(["pr", "review", &pr_number.to_string(), "--approve"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .context("Failed to approve PR")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to approve PR: {}", stderr);
        }
        Ok(())
    }

    /// Request changes on a PR
    pub fn request_changes(&self, pr_number: u64, body: &str) -> Result<()> {
        let output = Command::new("gh")
            .args(["pr", "review", &pr_number.to_string(), "--request-changes", "-b", body])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .context("Failed to request changes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to request changes: {}", stderr);
        }
        Ok(())
    }

    /// Add a comment to a PR (general review comment)
    pub fn comment_pr(&self, pr_number: u64, body: &str) -> Result<()> {
        let output = Command::new("gh")
            .args(["pr", "review", &pr_number.to_string(), "--comment", "-b", body])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .context("Failed to comment on PR")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to comment: {}", stderr);
        }
        Ok(())
    }

    /// Add a line comment to a PR
    pub fn add_line_comment(&self, pr_number: u64, path: &str, line: u32, body: &str) -> Result<()> {
        // Use gh api to create a review comment on a specific line
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{{owner}}/{{repo}}/pulls/{}/comments", pr_number),
                "-f", &format!("body={}", body),
                "-f", &format!("path={}", path),
                "-f", "commit_id=$(gh pr view --json headRefOid -q .headRefOid)",
                "-F", &format!("line={}", line),
                "-f", "side=RIGHT",
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .context("Failed to add line comment")?;

        if !output.status.success() {
            // Try alternative: get commit SHA first, then post comment
            let sha_output = Command::new("gh")
                .args(["pr", "view", &pr_number.to_string(), "--json", "headRefOid", "-q", ".headRefOid"])
                .output()
                .context("Failed to get PR head SHA")?;

            let commit_sha = String::from_utf8_lossy(&sha_output.stdout).trim().to_string();

            let output2 = Command::new("gh")
                .args([
                    "api",
                    "--method", "POST",
                    &format!("repos/{{owner}}/{{repo}}/pulls/{}/comments", pr_number),
                    "-f", &format!("body={}", body),
                    "-f", &format!("path={}", path),
                    "-f", &format!("commit_id={}", commit_sha),
                    "-F", &format!("line={}", line),
                    "-f", "side=RIGHT",
                ])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .output()
                .context("Failed to add line comment")?;

            if !output2.status.success() {
                let stderr = String::from_utf8_lossy(&output2.stderr);
                anyhow::bail!("Failed to add line comment: {}", stderr);
            }
        }
        Ok(())
    }
}

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new()
    }
}
