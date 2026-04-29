#![allow(dead_code)]

use anyhow::Result;

mod app;
mod diff;
mod review;
mod session;
mod ui;

fn main() -> Result<()> {
    app::run()
}
