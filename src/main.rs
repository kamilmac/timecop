use anyhow::Result;
use clap::Parser;

mod app;
mod diff;
mod review;
mod session;
mod ui;

#[derive(Parser, Debug)]
#[command(name = "timecop", about = "AI-native code review TUI", version)]
struct Cli {
    /// PR number or branch name; omit for current branch
    target: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    app::run(cli.target)
}
