pub enum WrapperCommandResult {
    Help,
    Status,
    Print(String),
    Send {
        plan: String,
        agent: String,
        background: bool,
    },
    Stop {
        session_id: String,
    },
    Focus {
        session_id: String,
    },
    Detach,
    Runs,
    ResultCmd {
        run_id: String,
    },
    Tail {
        run_id: String,
        lines: usize,
    },
    StopAll,
}

pub fn run_wrapper_command(input: &str) -> WrapperCommandResult {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return WrapperCommandResult::Print("".to_string());
    }

    match parts[0] {
        "help" => WrapperCommandResult::Help,
        "status" => WrapperCommandResult::Status,
        "send" => {
            if parts.len() < 3 {
                return WrapperCommandResult::Print(
                    "usage: send <plan> <agent> [--bg]".to_string(),
                );
            }
            let plan = parts[1].to_string();
            let agent = parts[2].to_string();

            let mut background = false;
            if parts.len() >= 4 {
                if parts[3] == "--bg" {
                    background = true;
                } else {
                    return WrapperCommandResult::Print(format!("unknown option: {}", parts[3]));
                }
            }

            // Validate agent
            match agent.as_str() {
                "codex" | "claude" | "gemini" | "agy" => {}
                _ => return WrapperCommandResult::Print(format!("unknown agent: {}", agent)),
            }

            // Validate plan folder existence
            let plan_path = std::path::Path::new(".agentmux").join("plans").join(&plan);
            if !plan_path.exists() || !plan_path.is_dir() {
                return WrapperCommandResult::Print(format!("plan not found: {}", plan));
            }

            WrapperCommandResult::Send {
                plan,
                agent,
                background,
            }
        }
        "stop" => {
            if parts.len() < 2 {
                return WrapperCommandResult::Print("usage: stop <session-id>".to_string());
            }
            let session_id = parts[1].to_string();
            WrapperCommandResult::Stop { session_id }
        }
        "focus" => {
            if parts.len() < 2 {
                return WrapperCommandResult::Print("usage: focus <session-id>".to_string());
            }
            let session_id = parts[1].to_string();
            WrapperCommandResult::Focus { session_id }
        }
        "detach" => WrapperCommandResult::Detach,
        "runs" => WrapperCommandResult::Runs,
        "result" => {
            if parts.len() < 2 {
                return WrapperCommandResult::Print("usage: result <run-id>".to_string());
            }
            let run_id = parts[1].to_string();
            WrapperCommandResult::ResultCmd { run_id }
        }
        "tail" => {
            if parts.len() < 2 {
                return WrapperCommandResult::Print("usage: tail <run-id> [lines]".to_string());
            }
            let run_id = parts[1].to_string();
            let mut lines = 80;
            if parts.len() >= 3 {
                match parts[2].parse::<usize>() {
                    Ok(val) => {
                        if val == 0 {
                            return WrapperCommandResult::Print(
                                "usage: tail <run-id> [lines]".to_string(),
                            );
                        }
                        lines = val;
                    }
                    Err(_) => {
                        return WrapperCommandResult::Print(
                            "usage: tail <run-id> [lines]".to_string(),
                        );
                    }
                }
            }
            WrapperCommandResult::Tail { run_id, lines }
        }
        "stop-all" => WrapperCommandResult::StopAll,
        unknown => WrapperCommandResult::Print(format!("unknown agentmux command: {}", unknown)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_help_command() {
        assert!(matches!(
            run_wrapper_command("help"),
            WrapperCommandResult::Help
        ));
    }

    #[test]
    fn test_status_command() {
        assert!(matches!(
            run_wrapper_command("status"),
            WrapperCommandResult::Status
        ));
    }

    #[test]
    fn test_unknown_command() {
        match run_wrapper_command("invalid") {
            WrapperCommandResult::Print(msg) => {
                assert_eq!(msg, "unknown agentmux command: invalid");
            }
            _ => panic!("Expected Print"),
        }
    }

    #[test]
    fn test_missing_send_arguments() {
        match run_wrapper_command("send") {
            WrapperCommandResult::Print(msg) => {
                assert_eq!(msg, "usage: send <plan> <agent> [--bg]");
            }
            _ => panic!("Expected Print"),
        }
        match run_wrapper_command("send auth") {
            WrapperCommandResult::Print(msg) => {
                assert_eq!(msg, "usage: send <plan> <agent> [--bg]");
            }
            _ => panic!("Expected Print"),
        }
    }

    #[test]
    fn test_unknown_agent() {
        match run_wrapper_command("send auth invalid") {
            WrapperCommandResult::Print(msg) => {
                assert_eq!(msg, "unknown agent: invalid");
            }
            _ => panic!("Expected Print"),
        }
    }

    #[test]
    fn test_valid_send_missing_plan() {
        match run_wrapper_command("send auth_missing agy") {
            WrapperCommandResult::Print(msg) => {
                assert_eq!(msg, "plan not found: auth_missing");
            }
            _ => panic!("Expected Print"),
        }
    }

    #[test]
    fn test_valid_send_with_plan() {
        let plan_dir = std::path::Path::new(".agentmux")
            .join("plans")
            .join("auth_test");
        let _ = fs::create_dir_all(&plan_dir);

        match run_wrapper_command("send auth_test agy") {
            WrapperCommandResult::Send {
                plan,
                agent,
                background,
            } => {
                assert_eq!(plan, "auth_test");
                assert_eq!(agent, "agy");
                assert!(!background);
            }
            _ => panic!("Expected Send"),
        }

        let _ = fs::remove_dir_all(plan_dir);
    }

    #[test]
    fn test_background_send_with_plan() {
        let plan_dir = std::path::Path::new(".agentmux")
            .join("plans")
            .join("auth_bg_test");
        let _ = fs::create_dir_all(&plan_dir);

        match run_wrapper_command("send auth_bg_test agy --bg") {
            WrapperCommandResult::Send {
                plan,
                agent,
                background,
            } => {
                assert_eq!(plan, "auth_bg_test");
                assert_eq!(agent, "agy");
                assert!(background);
            }
            _ => panic!("Expected Send with background=true"),
        }

        let _ = fs::remove_dir_all(plan_dir);
    }

    #[test]
    fn test_stop_missing_args() {
        match run_wrapper_command("stop") {
            WrapperCommandResult::Print(msg) => {
                assert_eq!(msg, "usage: stop <session-id>");
            }
            _ => panic!("Expected Print"),
        }
    }

    #[test]
    fn test_stop_command() {
        match run_wrapper_command("stop auth-agy-123") {
            WrapperCommandResult::Stop { session_id } => {
                assert_eq!(session_id, "auth-agy-123");
            }
            _ => panic!("Expected Stop"),
        }
    }

    #[test]
    fn test_focus_missing_args() {
        match run_wrapper_command("focus") {
            WrapperCommandResult::Print(msg) => {
                assert_eq!(msg, "usage: focus <session-id>");
            }
            _ => panic!("Expected Print"),
        }
    }

    #[test]
    fn test_focus_command() {
        match run_wrapper_command("focus auth-agy-123") {
            WrapperCommandResult::Focus { session_id } => {
                assert_eq!(session_id, "auth-agy-123");
            }
            _ => panic!("Expected Focus"),
        }
    }

    #[test]
    fn test_detach_command() {
        assert!(matches!(
            run_wrapper_command("detach"),
            WrapperCommandResult::Detach
        ));
    }

    #[test]
    fn test_runs_command() {
        assert!(matches!(
            run_wrapper_command("runs"),
            WrapperCommandResult::Runs
        ));
    }

    #[test]
    fn test_result_command() {
        match run_wrapper_command("result test-run-id") {
            WrapperCommandResult::ResultCmd { run_id } => {
                assert_eq!(run_id, "test-run-id");
            }
            _ => panic!("Expected ResultCmd"),
        }
    }

    #[test]
    fn test_result_command_missing_args() {
        match run_wrapper_command("result") {
            WrapperCommandResult::Print(msg) => {
                assert_eq!(msg, "usage: result <run-id>");
            }
            _ => panic!("Expected Print"),
        }
    }

    #[test]
    fn test_tail_command_default_lines() {
        match run_wrapper_command("tail test-run-id") {
            WrapperCommandResult::Tail { run_id, lines } => {
                assert_eq!(run_id, "test-run-id");
                assert_eq!(lines, 80);
            }
            _ => panic!("Expected Tail"),
        }
    }

    #[test]
    fn test_tail_command_explicit_lines() {
        match run_wrapper_command("tail test-run-id 40") {
            WrapperCommandResult::Tail { run_id, lines } => {
                assert_eq!(run_id, "test-run-id");
                assert_eq!(lines, 40);
            }
            _ => panic!("Expected Tail"),
        }
    }

    #[test]
    fn test_tail_command_missing_run_id() {
        match run_wrapper_command("tail") {
            WrapperCommandResult::Print(msg) => {
                assert_eq!(msg, "usage: tail <run-id> [lines]");
            }
            _ => panic!("Expected Print"),
        }
    }

    #[test]
    fn test_tail_command_zero_lines() {
        match run_wrapper_command("tail test-run-id 0") {
            WrapperCommandResult::Print(msg) => {
                assert_eq!(msg, "usage: tail <run-id> [lines]");
            }
            _ => panic!("Expected Print"),
        }
    }

    #[test]
    fn test_tail_command_non_numeric_lines() {
        match run_wrapper_command("tail test-run-id abc") {
            WrapperCommandResult::Print(msg) => {
                assert_eq!(msg, "usage: tail <run-id> [lines]");
            }
            _ => panic!("Expected Print"),
        }
    }

    #[test]
    fn test_stop_all_command() {
        assert!(matches!(
            run_wrapper_command("stop-all"),
            WrapperCommandResult::StopAll
        ));
    }
}
