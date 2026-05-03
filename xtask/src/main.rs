use anyhow::{Context, Result, bail};
use std::env;
use std::io::ErrorKind;
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
        Some("audit") => audit(),
        Some("help") | Some("--help") | Some("-h") | None => {
            print_usage();
            Ok(())
        }
        Some(other) => bail!("unknown xtask command: {other}"),
    }
}

fn print_usage() {
    println!("usage: cargo xtask <ci|fmt|lint|test|audit>");
}

fn ci() -> Result<()> {
    fmt()?;
    lint()?;
    test()?;
    audit()?;
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

fn audit() -> Result<()> {
    ensure_tool("cargo-audit", "cargo install cargo-audit --locked")?;
    cargo(&["audit"])
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

fn ensure_tool(tool: &str, install_command: &str) -> Result<()> {
    match Command::new(tool)
        .arg("--version")
        .current_dir(workspace_root())
        .status()
    {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!(
            "{tool} is installed but not working (status {status}); reinstall with `{install_command}`"
        ),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            bail!("{tool} is required; install it with `{install_command}`")
        }
        Err(error) => Err(error).with_context(|| format!("failed to probe `{tool}`")),
    }
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should live in the workspace root")
}
