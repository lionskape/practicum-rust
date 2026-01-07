use anyhow::{Context, Result, bail};
use std::process::{Command, ExitStatus};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "help".to_string());

    match cmd.as_str() {
        "help" | "-h" | "--help" => help(),
        "fmt" => run("cargo", &["fmt", "--all"]),
        "fmt-check" => run("cargo", &["fmt", "--all", "--", "--check"]),
        "clippy" => run("cargo", &["clippy", "--workspace", "--", "-D", "warnings"]),
        "test" => run("cargo", &["test", "--workspace"]),
        "ci" => {
            run("cargo", &["fmt", "--all", "--", "--check"])?;
            run("cargo", &["clippy", "--workspace", "--", "-D", "warnings"])?;
            run("cargo", &["test", "--workspace"])?;
            Ok(())
        }
        other => bail!("Unknown command: {other}\n\nRun: cargo run -p xtask -- help"),
    }
}

fn help() -> Result<()> {
    println!(
        r#"xtask

Usage:
  cargo run -p xtask -- <command>

Commands:
  help         Show this message
  fmt          Run rustfmt
  fmt-check    Check formatting (CI)
  clippy       Run clippy (workspace)
  test         Run tests (workspace)
  ci           Run fmt-check + clippy + test
"#
    );
    Ok(())
}

fn run(bin: &str, args: &[&str]) -> Result<()> {
    eprintln!("> {bin} {}", args.join(" "));
    let status = Command::new(bin)
        .args(args)
        .status()
        .with_context(|| format!("failed to spawn {bin}"))?;

    ensure_success(bin, args, status)
}

fn ensure_success(bin: &str, args: &[&str], status: ExitStatus) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        bail!(
            "command failed: {bin} {}\nexit code: {}",
            args.join(" "),
            status.code().unwrap_or(-1)
        )
    }
}
