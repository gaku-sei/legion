//! Legion App
//!
//! This crate is about everything concerning the highest-level, application
//! layer of a Legion app.

// crate-specific lint exceptions:
#![allow(clippy::struct_excessive_bools)]

mod bench;
mod build;
mod cargo;
mod cd;
mod changed_since;
mod check;
mod ci;
mod clippy;
mod config;
mod context;
mod doc;
mod error;
mod fix;
mod fmt;
mod git;
mod hakari;
mod installer;
mod lint;
mod run;
mod term;
mod test;
mod tools;
mod vscode;

//mod sources;
//mod hash;
//mod package;

use clap::{Parser, Subcommand};
use lgn_tracing::{span_scope, LevelFilter};

use error::Error;

/// A convenience type alias to return `Error`s from functions.
pub type Result<T> = std::result::Result<T, Error>;

/// Legion CLI
#[derive(Parser)]
#[clap(name = "lgn-monorepo")]
#[clap(about = "Legion Monorepo CLI")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // Cargo commands:
    /// Run `cargo bench`
    #[clap(name = "bench")]
    Bench(bench::Args),
    /// Run `cargo build`
    // the argument must be Boxed due to it's size and clippy (it's quite large by comparison to others.)
    #[clap(name = "build")]
    Build(build::Args),
    /// Run `cargo check`
    #[clap(name = "check")]
    Check(check::Args),
    /// Run `cargo clippy`
    #[clap(name = "clippy")]
    Clippy(clippy::Args),
    /// Run `cargo doc`
    #[clap(name = "doc")]
    Doc(doc::Args),
    /// Only list the packages with changes since the specified Git reference
    /// Run `cargo fix`
    #[clap(name = "fix")]
    Fix(fix::Args),
    /// Run `cargo fmt`
    #[clap(name = "fmt")]
    Fmt(fmt::Args),
    /// Run `cargo run`
    #[clap(name = "run")]
    Run(run::Args),
    /// Run `cargo tests`
    #[clap(name = "test")]
    Test(test::Args),

    // Non Cargo commands:
    /// Run CD on the monorepo
    #[clap(name = "cd")]
    Cd(cd::Args),
    /// Run CI check, defaults to running all checks
    #[clap(name = "ci")]
    Ci(ci::Args),
    /// List packages changed since merge base with the given commit
    ///
    /// Note that this compares against the merge base (common ancestor) of the specified commit.
    /// For example, if origin/master is specified, the current working directory will be compared
    /// against the point at which it branched off of origin/master.
    #[clap(name = "changed-since")]
    ChangedSince(changed_since::Args),
    /// Generate the workspace hack dependencies
    #[clap(name = "hakari")]
    Hakari,
    /// Run tools installation
    #[clap(name = "lint")]
    Lint(lint::Args),
    /// Run tools installation
    #[clap(name = "tools")]
    Tools(tools::Args),
    /// Generates VSCode configuration files and performs checks
    #[clap(name = "vscode")]
    VsCode(vscode::Args),
}

fn main() {
    let _telemetry_guard = lgn_telemetry_sink::TelemetryGuard::default()
        .unwrap()
        .with_log_level(LevelFilter::Warn);

    span_scope!("monorepo::main");

    let args = Cli::parse();
    if let Err(err) = context::Context::new().and_then(|ctx| match args.command {
        Commands::Build(args) => build::run(args, &ctx),
        Commands::Bench(args) => bench::run(args, &ctx),
        Commands::Check(args) => check::run(&args, &ctx),
        Commands::Clippy(args) => clippy::run(&args, &ctx),
        Commands::Doc(args) => doc::run(args, &ctx),
        Commands::Fix(args) => fix::run(args, &ctx),
        Commands::Fmt(args) => fmt::run(args, &ctx),
        Commands::Run(args) => run::run(&args, &ctx),
        Commands::Test(args) => test::run(args, &ctx),

        Commands::Cd(args) => cd::run(&args, &ctx),
        Commands::Ci(args) => ci::run(&args, &ctx),
        Commands::ChangedSince(args) => changed_since::run(&args, &ctx),
        Commands::Hakari => hakari::run(&ctx),
        Commands::Lint(args) => lint::run(&args, &ctx),
        Commands::Tools(args) => tools::run(&args, &ctx),
        Commands::VsCode(args) => vscode::run(&args, &ctx),
    }) {
        err.display();
        #[allow(clippy::exit)]
        std::process::exit(err.exit_code().unwrap_or(1));
    }
}
