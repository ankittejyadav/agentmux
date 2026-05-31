mod commands;
mod config;
mod pty;
mod runs;
mod sessions;
mod waiting;

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

const DEFAULT_SESSIONS: &str = "[]\n";

fn main() {
    if let Err(error) = run() {
        eprintln!("agentmux: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return Ok(());
    };

    match command.as_str() {
        "init" => {
            ensure_agentmux_state(&env::current_dir().map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())?;
            println!("initialized .agentmux");
        }
        "plan" => {
            let Some(name) = args.next() else {
                return Err("usage: agentmux plan <name>".to_string());
            };

            create_plan(
                &env::current_dir().map_err(|error| error.to_string())?,
                &name,
            )
            .map_err(|error| error.to_string())?;
            println!("created plan {name}");
        }
        "codex" | "claude" | "gemini" | "agy" => {
            let mut current_agent = command;
            let mut current_args = args.collect::<Vec<String>>();
            let mut startup_prompt = None;

            loop {
                match pty::run_pty_command(&current_agent, current_args, startup_prompt)? {
                    pty::SessionAction::Exit(code) => process::exit(code),
                    pty::SessionAction::SwitchAgent { agent, prompt } => {
                        current_agent = agent;
                        current_args = Vec::new();
                        startup_prompt = Some(prompt);
                    }
                }
            }
        }
        "cleanup-runs" => {
            let current_args = args.collect::<Vec<String>>();
            crate::runs::cleanup_runs(&current_args)?;
        }
        "help" | "--help" | "-h" => print_help(),
        "version" | "--version" | "-V" => println!("agentmux {}", env!("CARGO_PKG_VERSION")),
        unknown => return Err(format!("unknown command `{unknown}`")),
    }

    Ok(())
}

fn ensure_agentmux_state(cwd: &Path) -> io::Result<()> {
    let root = cwd.join(".agentmux");

    fs::create_dir_all(root.join("plans"))?;
    fs::create_dir_all(root.join("runs"))?;
    write_file_if_missing(&root.join("config.toml"), crate::config::DEFAULT_CONFIG)?;
    write_file_if_missing(&root.join("sessions.json"), DEFAULT_SESSIONS)?;

    Ok(())
}

fn create_plan(cwd: &Path, name: &str) -> io::Result<()> {
    validate_plan_name(name)?;
    ensure_agentmux_state(cwd)?;

    let plan_dir = cwd.join(".agentmux").join("plans").join(name);
    fs::create_dir_all(&plan_dir)?;

    write_file_if_missing(
        &plan_dir.join("plan.md"),
        &format!("# Plan: {name}\n\n## Goal\n\n## Architecture\n\n## Notes\n"),
    )?;
    write_file_if_missing(
        &plan_dir.join("tasks.md"),
        &format!("# Tasks: {name}\n\n- [ ] Task 1\n"),
    )?;
    write_file_if_missing(
        &plan_dir.join("acceptance.md"),
        &format!("# Acceptance: {name}\n\n- [ ] Verification is documented\n"),
    )?;
    write_file_if_missing(
        &plan_dir.join("constraints.md"),
        &format!("# Constraints: {name}\n\n- Keep native harness behavior intact\n"),
    )?;

    Ok(())
}

fn validate_plan_name(name: &str) -> io::Result<()> {
    let invalid =
        name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\');

    if invalid {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "plan name must be a simple folder name",
        ))
    } else {
        Ok(())
    }
}

fn write_file_if_missing(path: &PathBuf, content: &str) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }

    fs::write(path, content)
}

fn print_help() {
    println!(
        r#"agentmux

Usage:
  agentmux init
  agentmux plan <name>
  agentmux codex|claude|gemini|agy

Commands:
  init              create .agentmux project state
  plan <name>       create a plan folder and starter docs
  codex             launch native Codex CLI
  claude            launch native Claude CLI
  gemini            launch native Gemini CLI
  agy               launch native Antigravity CLI
  cleanup-runs      remove older run logs
  help              show this help
  version           show the version
"#
    );
}
