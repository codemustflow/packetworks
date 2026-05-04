use anyhow::{Context, Result, bail};
use std::env;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, ExitCode, Stdio};
use std::thread;
use std::time::Duration;

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
        Some("p2p") => p2p(),
        Some("help") | Some("--help") | Some("-h") | None => {
            print_usage();
            Ok(())
        }
        Some(other) => bail!("unknown xtask command: {other}"),
    }
}

fn print_usage() {
    println!("usage: cargo xtask <ci|fmt|lint|test|build-all|p2p>");
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

fn p2p() -> Result<()> {
    let mut sink = cargo_child(
        "sink",
        &[
            ("LOG_FOR_HUMANS", "true"),
            ("LOG_LEVEL", "info"),
            ("BIND_ADDRESS", "127.0.0.1"),
            ("BIND_PORT", "9101"),
            ("STATS_INTERVAL_SECONDS", "1"),
        ],
    )?;

    let sink_stdout = sink
        .stdout
        .take()
        .context("failed to capture sink stdout")?;
    let sink_stderr = sink
        .stderr
        .take()
        .context("failed to capture sink stderr")?;

    let sink_stdout_thread = stream_output("sink", sink_stdout);
    let sink_stderr_thread = stream_output("sink", sink_stderr);

    thread::sleep(Duration::from_millis(200));

    let mut source = cargo_child(
        "source",
        &[
            ("LOG_FOR_HUMANS", "true"),
            ("LOG_LEVEL", "info"),
            ("BIND_ADDRESS", "127.0.0.1"),
            ("BIND_PORT", "9102"),
            ("PEER_ADDRESS", "127.0.0.1"),
            ("PEER_PORT", "9101"),
            ("STATS_INTERVAL_SECONDS", "1"),
        ],
    )?;

    let source_stdout = source
        .stdout
        .take()
        .context("failed to capture source stdout")?;
    let source_stderr = source
        .stderr
        .take()
        .context("failed to capture source stderr")?;

    let source_stdout_thread = stream_output("source", source_stdout);
    let source_stderr_thread = stream_output("source", source_stderr);

    monitor_p2p(&mut sink, &mut source)?;

    join_output_thread("sink stdout", sink_stdout_thread)?;
    join_output_thread("sink stderr", sink_stderr_thread)?;
    join_output_thread("source stdout", source_stdout_thread)?;
    join_output_thread("source stderr", source_stderr_thread)?;

    Ok(())
}

fn cargo(args: &[&str]) -> Result<()> {
    run("cargo", args)
}

fn cargo_child(binary: &str, envs: &[(&str, &str)]) -> Result<Child> {
    eprintln!("+ cargo run --quiet --locked -p {binary}");

    let mut command = Command::new("cargo");
    command
        .args(["run", "--quiet", "--locked", "-p", binary])
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, value) in envs {
        command.env(key, value);
    }

    command
        .spawn()
        .with_context(|| format!("failed to spawn `cargo run -p {binary}`"))
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

fn monitor_p2p(sink: &mut Child, source: &mut Child) -> Result<()> {
    loop {
        if let Some(status) = sink.try_wait().context("failed to poll sink process")? {
            terminate_child(source, "source")?;

            if status.success() {
                return Ok(());
            }

            bail!("sink exited with status {status}");
        }

        if let Some(status) = source.try_wait().context("failed to poll source process")? {
            terminate_child(sink, "sink")?;

            if status.success() {
                return Ok(());
            }

            bail!("source exited with status {status}");
        }

        thread::sleep(Duration::from_millis(100));
    }
}

fn terminate_child(child: &mut Child, name: &str) -> Result<()> {
    if child
        .try_wait()
        .with_context(|| format!("failed to poll {name} process"))?
        .is_some()
    {
        return Ok(());
    }

    child
        .kill()
        .with_context(|| format!("failed to terminate {name} process"))?;
    child
        .wait()
        .with_context(|| format!("failed to wait for {name} process after termination"))?;

    Ok(())
}

fn stream_output<T>(label: &'static str, reader: T) -> thread::JoinHandle<Result<()>>
where
    T: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let reader = BufReader::new(reader);

        for line in reader.lines() {
            println!(
                "[{label}] {}",
                line.with_context(|| format!("failed to read {label} output"))?
            );
        }

        Ok(())
    })
}

fn join_output_thread(label: &str, handle: thread::JoinHandle<Result<()>>) -> Result<()> {
    handle
        .join()
        .map_err(|_| anyhow::anyhow!("{label} thread panicked"))??;

    Ok(())
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should live in the workspace root")
}
