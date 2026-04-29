use crate::diff;
use crate::diff::types::Diff;
use crate::session::{ExistingComment, Overlay, Reactions, Thread, Verdict};
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

pub struct PrSession {
    pub number: u32,
    pub owner: String,
    pub repo: String,
    pub head_ref: String,
    pub base_ref: String,
    pub head_sha: String,
    pub diff: Diff,
    pub overlay: Overlay,
}

#[derive(Deserialize)]
struct PrView {
    number: u32,
    body: String,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
    #[serde(rename = "baseRefName")]
    base_ref_name: String,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
}

#[derive(Deserialize)]
struct RepoView {
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
}

#[derive(Deserialize, Debug)]
struct ReviewComment {
    id: u64,
    path: String,
    line: Option<u32>,
    #[serde(default)]
    original_line: Option<u32>,
    in_reply_to_id: Option<u64>,
    body: String,
    user: User,
    created_at: String,
    #[serde(default)]
    reactions: ReactionsRaw,
}

#[derive(Deserialize, Debug)]
struct User {
    login: String,
}

#[derive(Deserialize, Debug, Default)]
struct ReactionsRaw {
    #[serde(default, rename = "+1")]
    thumbs_up: u32,
    #[serde(default, rename = "-1")]
    thumbs_down: u32,
    #[serde(default)]
    heart: u32,
    #[serde(default)]
    hooray: u32,
    #[serde(default)]
    laugh: u32,
    #[serde(default)]
    confused: u32,
    #[serde(default)]
    rocket: u32,
    #[serde(default)]
    eyes: u32,
}

impl From<&ReactionsRaw> for Reactions {
    fn from(r: &ReactionsRaw) -> Self {
        Reactions {
            thumbs_up: r.thumbs_up,
            thumbs_down: r.thumbs_down,
            heart: r.heart,
            hooray: r.hooray,
            laugh: r.laugh,
            confused: r.confused,
            rocket: r.rocket,
            eyes: r.eyes,
        }
    }
}

#[derive(Deserialize)]
struct GraphqlResp {
    data: GraphqlData,
}

#[derive(Deserialize)]
struct GraphqlData {
    repository: GraphqlRepo,
}

#[derive(Deserialize)]
struct GraphqlRepo {
    #[serde(rename = "pullRequest")]
    pull_request: GraphqlPr,
}

#[derive(Deserialize)]
struct GraphqlPr {
    #[serde(rename = "reviewThreads")]
    review_threads: GraphqlThreadsConn,
}

#[derive(Deserialize)]
struct GraphqlThreadsConn {
    nodes: Vec<GraphqlThread>,
}

#[derive(Deserialize)]
struct GraphqlThread {
    id: String,
    #[serde(rename = "isResolved")]
    is_resolved: bool,
    comments: GraphqlCommentsConn,
}

#[derive(Deserialize)]
struct GraphqlCommentsConn {
    nodes: Vec<GraphqlComment>,
}

#[derive(Deserialize)]
struct GraphqlComment {
    #[serde(rename = "databaseId")]
    database_id: u64,
}

pub fn open(pr_number: u32) -> Result<PrSession> {
    let view: PrView = run_gh_json(&[
        "pr",
        "view",
        &pr_number.to_string(),
        "--json",
        "number,body,headRefName,baseRefName,headRefOid",
    ])?;

    let repo_view: RepoView = run_gh_json(&["repo", "view", "--json", "nameWithOwner"])?;
    let (owner, repo) = repo_view
        .name_with_owner
        .split_once('/')
        .map(|(o, r)| (o.to_string(), r.to_string()))
        .ok_or_else(|| anyhow!("malformed nameWithOwner: {}", repo_view.name_with_owner))?;

    let unified = run_gh_text(&["pr", "diff", &pr_number.to_string()])?;
    let diff = diff::parse::parse(&unified)?;

    let comments: Vec<ReviewComment> = run_gh_json(&[
        "api",
        &format!("repos/{owner}/{repo}/pulls/{pr_number}/comments?per_page=100"),
    ])?;

    let resolved_map = fetch_resolution_map(&owner, &repo, pr_number).unwrap_or_default();

    let threads = build_threads(comments, &resolved_map);

    let overlay = Overlay {
        description: view.body.clone(),
        threads,
    };

    Ok(PrSession {
        number: view.number,
        owner,
        repo,
        head_ref: view.head_ref_name,
        base_ref: view.base_ref_name,
        head_sha: view.head_ref_oid,
        diff,
        overlay,
    })
}

fn build_threads(
    comments: Vec<ReviewComment>,
    resolved_map: &HashMap<u64, (String, bool)>,
) -> Vec<Thread> {
    let mut by_id: HashMap<u64, &ReviewComment> = HashMap::new();
    for c in &comments {
        by_id.insert(c.id, c);
    }

    let roots: Vec<&ReviewComment> = comments
        .iter()
        .filter(|c| c.in_reply_to_id.is_none())
        .collect();

    let mut threads: Vec<Thread> = Vec::new();
    for root in roots {
        let mut thread_comments: Vec<&ReviewComment> = vec![root];
        for c in &comments {
            if c.in_reply_to_id == Some(root.id) {
                thread_comments.push(c);
            }
        }
        thread_comments.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let outdated = root.line.is_none();
        let line = root.line.or(root.original_line).unwrap_or(0);

        let (node_id, resolved) = resolved_map
            .get(&root.id)
            .cloned()
            .unwrap_or_else(|| (String::new(), false));

        threads.push(Thread {
            node_id,
            root_comment_id: root.id,
            file: root.path.clone(),
            line,
            comments: thread_comments
                .into_iter()
                .map(|c| ExistingComment {
                    id: c.id,
                    author: c.user.login.clone(),
                    created_at: c.created_at.clone(),
                    body: c.body.clone(),
                    reactions: Reactions::from(&c.reactions),
                })
                .collect(),
            resolved,
            outdated,
        });
    }

    threads
}

fn fetch_resolution_map(
    owner: &str,
    repo: &str,
    pr: u32,
) -> Result<HashMap<u64, (String, bool)>> {
    let query = format!(
        r#"query {{ repository(owner: "{owner}", name: "{repo}") {{ pullRequest(number: {pr}) {{ reviewThreads(first: 100) {{ nodes {{ id isResolved comments(first: 1) {{ nodes {{ databaseId }} }} }} }} }} }} }}"#
    );
    let resp: GraphqlResp = run_gh_json(&["api", "graphql", "-f", &format!("query={query}")])?;
    let mut map = HashMap::new();
    for t in resp.data.repository.pull_request.review_threads.nodes {
        if let Some(first) = t.comments.nodes.first() {
            map.insert(first.database_id, (t.id, t.is_resolved));
        }
    }
    Ok(map)
}

pub fn post_comment(session: &PrSession, file: &str, line: u32, body: &str) -> Result<()> {
    let path = format!(
        "repos/{}/{}/pulls/{}/comments",
        session.owner, session.repo, session.number
    );
    let body_json = serde_json::json!({
        "commit_id": session.head_sha,
        "path": file,
        "line": line,
        "side": "RIGHT",
        "body": body,
    });
    gh_post_json(&["api", "-X", "POST", &path, "--input", "-"], &body_json)?;
    Ok(())
}

pub fn post_reaction(session: &PrSession, comment_id: u64, content: &str) -> Result<()> {
    let path = format!(
        "repos/{}/{}/pulls/comments/{}/reactions",
        session.owner, session.repo, comment_id
    );
    let body = serde_json::json!({ "content": content });
    gh_post_json(&["api", "-X", "POST", &path, "--input", "-"], &body)?;
    Ok(())
}

pub fn post_reply(session: &PrSession, root_comment_id: u64, body: &str) -> Result<()> {
    let path = format!(
        "repos/{}/{}/pulls/{}/comments/{}/replies",
        session.owner, session.repo, session.number, root_comment_id
    );
    let body_json = serde_json::json!({ "body": body });
    gh_post_json(&["api", "-X", "POST", &path, "--input", "-"], &body_json)?;
    Ok(())
}

pub fn submit_verdict(session: &PrSession, verdict: Verdict) -> Result<()> {
    let event = match verdict {
        Verdict::Approve => "APPROVE",
        Verdict::RequestChanges => "REQUEST_CHANGES",
    };
    let body = serde_json::json!({
        "commit_id": session.head_sha,
        "event": event,
        "body": "",
    });
    let path = format!(
        "repos/{}/{}/pulls/{}/reviews",
        session.owner, session.repo, session.number
    );
    gh_post_json(&["api", "-X", "POST", &path, "--input", "-"], &body)?;
    Ok(())
}

pub fn set_resolved(thread_node_id: &str, resolved: bool) -> Result<()> {
    let mutation_name = if resolved {
        "resolveReviewThread"
    } else {
        "unresolveReviewThread"
    };
    let mutation = format!(
        r#"mutation {{ {mutation_name}(input: {{threadId: "{thread_node_id}"}}) {{ thread {{ isResolved }} }} }}"#
    );
    run_gh_text(&["api", "graphql", "-f", &format!("query={mutation}")])?;
    Ok(())
}

fn gh_post_json(args: &[&str], body: &serde_json::Value) -> Result<String> {
    let mut child = Command::new("gh")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn gh")?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("gh stdin"))?;
        stdin.write_all(body.to_string().as_bytes())?;
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Err(anyhow!(
            "gh {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn run_gh_text(args: &[&str]) -> Result<String> {
    let out = Command::new("gh")
        .args(args)
        .output()
        .context("spawn gh")?;
    if !out.status.success() {
        return Err(anyhow!(
            "gh {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn run_gh_json<T: serde::de::DeserializeOwned>(args: &[&str]) -> Result<T> {
    let text = run_gh_text(args)?;
    serde_json::from_str(&text)
        .with_context(|| format!("parse JSON from `gh {}`", args.join(" ")))
}
