//! rusty-whale - agent harness entry point
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rusty-whale", version, about = "Community-driven agent harness")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start an agent loop with the given prompt
    Run { prompt: String },
    /// List configured providers
    Providers,
    /// Verify a change set (build/test/clippy)
    Verify,
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd.unwrap_or(Cmd::Verify) {
        Cmd::Run { prompt } => println!("[run] {prompt} (stub)"),
        Cmd::Providers => println!("[providers] (stub)"),
        Cmd::Verify => println!("[verify] cargo build + clippy + test (stub)"),
    }
}
