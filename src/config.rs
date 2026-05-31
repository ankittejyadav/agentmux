use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const DEFAULT_CONFIG: &str = r#"command_prefix = "//"

[agents.codex]
command = "codex"
args = []

[agents.claude]
command = "claude"
args = []

[agents.gemini]
command = "gemini"
args = []

[agents.agy]
command = "agy"
args = []
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLaunch {
    pub key: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ConfigToml {
    command_prefix: Option<String>,
    agents: Option<HashMap<String, AgentConfig>>,
}

#[derive(Debug, Deserialize)]
struct AgentConfig {
    command: Option<String>,
    args: Option<Vec<String>>,
}

pub fn load_command_prefix() -> Result<String, String> {
    let path = Path::new(".agentmux").join("config.toml");
    load_command_prefix_from_path(&path)
}

fn load_command_prefix_from_path(path: &Path) -> Result<String, String> {
    if path.exists() && path.is_file() {
        let content = fs::read_to_string(path)
            .map_err(|err| format!("failed to read config file: {}", err))?;
        parse_command_prefix(&content)
    } else {
        parse_command_prefix(DEFAULT_CONFIG)
    }
}

fn parse_command_prefix(config_text: &str) -> Result<String, String> {
    let config: ConfigToml = toml::from_str(config_text)
        .map_err(|err| format!("invalid .agentmux/config.toml: {}", err))?;

    let prefix = config
        .command_prefix
        .ok_or_else(|| "command_prefix is missing".to_string())?;

    if prefix.is_empty() {
        return Err("command_prefix is empty".to_string());
    }

    if !prefix.is_ascii() {
        return Err("command_prefix must be ASCII".to_string());
    }

    Ok(prefix)
}

pub fn load_agent_launch(agent_key: &str) -> Result<AgentLaunch, String> {
    let path = Path::new(".agentmux").join("config.toml");
    load_agent_launch_from_path(&path, agent_key)
}

fn load_agent_launch_from_path(path: &Path, agent_key: &str) -> Result<AgentLaunch, String> {
    if path.exists() && path.is_file() {
        let content = fs::read_to_string(path)
            .map_err(|err| format!("failed to read config file: {}", err))?;
        parse_agent_launch(&content, agent_key)
    } else {
        parse_agent_launch(DEFAULT_CONFIG, agent_key)
    }
}

fn parse_agent_launch(config_text: &str, agent_key: &str) -> Result<AgentLaunch, String> {
    let config: ConfigToml = toml::from_str(config_text)
        .map_err(|err| format!("invalid .agentmux/config.toml: {}", err))?;

    let agents = config
        .agents
        .ok_or_else(|| format!("agent config missing: {}", agent_key))?;

    let agent_conf = agents
        .get(agent_key)
        .ok_or_else(|| format!("agent config missing: {}", agent_key))?;

    let command = agent_conf
        .command
        .clone()
        .ok_or_else(|| format!("agent config missing command for: {}", agent_key))?;

    if command.trim().is_empty() {
        return Err(format!("agent command is empty: {}", agent_key));
    }

    let args = agent_conf.args.clone().unwrap_or_default();

    Ok(AgentLaunch {
        key: agent_key.to_string(),
        command,
        args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_loads_codex() {
        let launch = parse_agent_launch(DEFAULT_CONFIG, "codex").unwrap();
        assert_eq!(launch.key, "codex");
        assert_eq!(launch.command, "codex");
        assert!(launch.args.is_empty());
    }

    #[test]
    fn test_custom_config_loads_absolute_and_args() {
        let config_text = r#"
[agents.codex]
command = "/usr/bin/custom-codex"
args = ["--model", "gpt-4"]
"#;
        let launch = parse_agent_launch(config_text, "codex").unwrap();
        assert_eq!(launch.key, "codex");
        assert_eq!(launch.command, "/usr/bin/custom-codex");
        assert_eq!(
            launch.args,
            vec!["--model".to_string(), "gpt-4".to_string()]
        );
    }

    #[test]
    fn test_missing_agent() {
        let config_text = r#"
[agents.codex]
command = "codex"
"#;
        let err = parse_agent_launch(config_text, "claude").unwrap_err();
        assert_eq!(err, "agent config missing: claude");
    }

    #[test]
    fn test_empty_command() {
        let config_text = r#"
[agents.codex]
command = ""
"#;
        let err = parse_agent_launch(config_text, "codex").unwrap_err();
        assert_eq!(err, "agent command is empty: codex");
    }

    #[test]
    fn test_invalid_toml() {
        let config_text = r#"
[agents.codex
command = "codex"
"#;
        let err = parse_agent_launch(config_text, "codex").unwrap_err();
        assert!(err.to_string().contains("invalid .agentmux/config.toml"));
    }

    #[test]
    fn test_invalid_toml_for_prefix() {
        let config_text = r#"
command_prefix = [
"#;
        let err = parse_command_prefix(config_text).unwrap_err();
        assert!(err.to_string().contains("invalid .agentmux/config.toml"));
    }

    #[test]
    fn test_default_prefix() {
        let prefix = parse_command_prefix(DEFAULT_CONFIG).unwrap();
        assert_eq!(prefix, "//");
    }

    #[test]
    fn test_custom_prefix() {
        let config_text = r#"command_prefix = "++""#;
        let prefix = parse_command_prefix(config_text).unwrap();
        assert_eq!(prefix, "++");
    }

    #[test]
    fn test_empty_prefix() {
        let config_text = r#"command_prefix = """#;
        let err = parse_command_prefix(config_text).unwrap_err();
        assert_eq!(err, "command_prefix is empty");
    }

    #[test]
    fn test_non_ascii_prefix() {
        let config_text = r#"command_prefix = "🦀""#;
        let err = parse_command_prefix(config_text).unwrap_err();
        assert_eq!(err, "command_prefix must be ASCII");
    }
}
