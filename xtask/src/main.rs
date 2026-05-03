use anyhow::{Context, Result, bail};
use std::env;
use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> Result<()> {
    let command = env::args().nth(1);

    match command.as_deref() {
        Some("ci") => ci(),
        Some("fmt") => fmt(),
        Some("lint") => lint(),
        Some("test") => test(),
        Some("build-all") => build_all(),
        Some("help") | Some("--help") | Some("-h") | None => {
            print_usage();
            Ok(())
        }
        Some(other) => bail!("unknown xtask command: {other}"),
    }
}

fn print_usage() {
    println!("usage: cargo xtask <ci|fmt|lint|test|build-all>");
}

fn ci() -> Result<()> {
    fmt()?;
    lint()?;
    test()?;
    build_all()?;
    Ok(())
}

fn fmt() -> Result<()> {
    cargo(&["fmt", "--all", "--", "--check"])
}

fn lint() -> Result<()> {
    cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings",
    ])
}

fn test() -> Result<()> {
    cargo(&["test", "--workspace", "--all-features", "--locked"])
}

fn build_all() -> Result<()> {
    cargo(&[
        "build",
        "--workspace",
        "--exclude",
        "xtask",
        "--all-features",
        "--locked",
    ])?;

    cargo(&[
        "build",
        "--workspace",
        "--exclude",
        "xtask",
        "--all-features",
        "--locked",
        "--release",
    ])
}

fn cargo(args: &[&str]) -> Result<()> {
    run("cargo", args)
}

fn run(program: &str, args: &[&str]) -> Result<()> {
    eprintln!("+ {program} {}", args.join(" "));

    let status = Command::new(program)
        .args(args)
        .current_dir(workspace_root())
        .status()
        .with_context(|| format!("failed to spawn `{program}`"))?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "command failed with status {status}: {program} {}",
            args.join(" ")
        )
    }
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should live in the workspace root")
}
