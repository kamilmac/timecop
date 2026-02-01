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

    /// Get PR info for the current branch
    pub fn get_pr_for_branch(&mut self, _branch: &str) -> Result<Option<PrInfo>> {
        if !self.is_available() {
            return Ok(None);
        }

        // Get PR number
        let output = Command::new("gh")
            .args(["pr", "view", "--json", "number,title,body,author,state,url"])
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

        let basic: PrBasic = serde_json::from_slice(&output.stdout)
            .context("Failed to parse PR JSON")?;

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

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new()
    }
}
