struct WaitingPattern {
    hint: &'static str,
    needle: &'static str,
}

const CODEX_PATTERNS: &[WaitingPattern] = &[
    WaitingPattern {
        hint: "approval",
        needle: "allow command",
    },
    WaitingPattern {
        hint: "approval",
        needle: "allow codex",
    },
    WaitingPattern {
        hint: "approval",
        needle: "approve command",
    },
    WaitingPattern {
        hint: "approval",
        needle: "requires approval",
    },
    WaitingPattern {
        hint: "permission",
        needle: "sandbox",
    },
    WaitingPattern {
        hint: "permission",
        needle: "escalated permissions",
    },
    WaitingPattern {
        hint: "permission",
        needle: "run outside the sandbox",
    },
];

const CLAUDE_PATTERNS: &[WaitingPattern] = &[
    WaitingPattern {
        hint: "permission",
        needle: "do you want to allow",
    },
    WaitingPattern {
        hint: "permission",
        needle: "allow this command",
    },
    WaitingPattern {
        hint: "permission",
        needle: "permission to use",
    },
    WaitingPattern {
        hint: "confirmation",
        needle: "continue?",
    },
    WaitingPattern {
        hint: "confirmation",
        needle: "proceed?",
    },
];

const GEMINI_PATTERNS: &[WaitingPattern] = &[
    WaitingPattern {
        hint: "confirmation",
        needle: "do you want to continue",
    },
    WaitingPattern {
        hint: "confirmation",
        needle: "continue?",
    },
    WaitingPattern {
        hint: "confirmation",
        needle: "proceed?",
    },
    WaitingPattern {
        hint: "input",
        needle: "select an option",
    },
    WaitingPattern {
        hint: "input",
        needle: "press enter",
    },
];

const AGY_PATTERNS: &[WaitingPattern] = &[
    WaitingPattern {
        hint: "approval",
        needle: "approval required",
    },
    WaitingPattern {
        hint: "approval",
        needle: "requires approval",
    },
    WaitingPattern {
        hint: "permission",
        needle: "permission required",
    },
    WaitingPattern {
        hint: "permission",
        needle: "allow",
    },
    WaitingPattern {
        hint: "input",
        needle: "waiting for input",
    },
    WaitingPattern {
        hint: "input",
        needle: "press enter",
    },
];

pub fn detect_waiting_hint(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();

    // approval
    if lower.contains("requires approval")
        || lower.contains("approval")
        || lower.contains("approve")
    {
        return Some("approval");
    }

    // confirmation
    if lower.contains("proceed?") || lower.contains("continue?") || lower.contains("confirm") {
        return Some("confirmation");
    }

    // permission
    if lower.contains("permission") || lower.contains("allow") || lower.contains("do you want to") {
        return Some("permission");
    }

    // input
    if lower.contains("press enter")
        || lower.contains("select an option")
        || lower.contains("waiting for input")
    {
        return Some("input");
    }

    None
}

pub fn detect_waiting_hint_for_agent(agent: &str, text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    let patterns = match agent {
        "codex" => CODEX_PATTERNS,
        "claude" => CLAUDE_PATTERNS,
        "gemini" => GEMINI_PATTERNS,
        "agy" => AGY_PATTERNS,
        _ => &[],
    };

    for pat in patterns {
        if lower.contains(pat.needle) {
            return Some(pat.hint);
        }
    }

    // fallback to generic
    detect_waiting_hint(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_waiting_hint_approval() {
        assert_eq!(
            detect_waiting_hint("Requires approval before continuing"),
            Some("approval")
        );
        assert_eq!(
            detect_waiting_hint("Please approve this action"),
            Some("approval")
        );
        assert_eq!(
            detect_waiting_hint("Waiting for your approval"),
            Some("approval")
        );
    }

    #[test]
    fn test_detect_waiting_hint_permission() {
        assert_eq!(
            detect_waiting_hint("Need permission to write file"),
            Some("permission")
        );
        assert_eq!(
            detect_waiting_hint("Do you allow this tool to run?"),
            Some("permission")
        );
        assert_eq!(
            detect_waiting_hint("Do you want to run this?"),
            Some("permission")
        );
    }

    #[test]
    fn test_detect_waiting_hint_confirmation() {
        assert_eq!(
            detect_waiting_hint("Do you want to proceed?"),
            Some("confirmation")
        );
        assert_eq!(
            detect_waiting_hint("Should we continue?"),
            Some("confirmation")
        );
        assert_eq!(detect_waiting_hint("Please confirm"), Some("confirmation"));
    }

    #[test]
    fn test_detect_waiting_hint_input() {
        assert_eq!(
            detect_waiting_hint("Press Enter to continue"),
            Some("input")
        );
        assert_eq!(
            detect_waiting_hint("Select an option from the list"),
            Some("input")
        );
        assert_eq!(detect_waiting_hint("Waiting for input..."), Some("input"));
    }

    #[test]
    fn test_detect_waiting_hint_none() {
        assert_eq!(detect_waiting_hint("Compiling project..."), None);
        assert_eq!(detect_waiting_hint("Build finished successfully"), None);
    }

    #[test]
    fn test_codex_specific_patterns() {
        assert_eq!(
            detect_waiting_hint_for_agent("codex", "Should we allow command to run?"),
            Some("approval")
        );
        assert_eq!(
            detect_waiting_hint_for_agent("codex", "Requires escalated permissions for execution"),
            Some("permission")
        );
        assert_eq!(
            detect_waiting_hint_for_agent("codex", "Run outside the sandbox?"),
            Some("permission")
        );
    }

    #[test]
    fn test_claude_specific_patterns() {
        assert_eq!(
            detect_waiting_hint_for_agent("claude", "Do you want to allow this modification?"),
            Some("permission")
        );
        assert_eq!(
            detect_waiting_hint_for_agent("claude", "No permission to use tool"),
            Some("permission")
        );
    }

    #[test]
    fn test_gemini_specific_patterns() {
        assert_eq!(
            detect_waiting_hint_for_agent("gemini", "Do you want to continue processing?"),
            Some("confirmation")
        );
        assert_eq!(
            detect_waiting_hint_for_agent("gemini", "Select an option from below"),
            Some("input")
        );
    }

    #[test]
    fn test_agy_specific_patterns() {
        assert_eq!(
            detect_waiting_hint_for_agent("agy", "Action approval required"),
            Some("approval")
        );
        assert_eq!(
            detect_waiting_hint_for_agent("agy", "Ask to allow the operation"),
            Some("permission")
        );
    }

    #[test]
    fn test_unknown_agent_fallback() {
        assert_eq!(
            detect_waiting_hint_for_agent("unknown", "Requires approval before continuing"),
            Some("approval")
        );
        assert_eq!(
            detect_waiting_hint_for_agent("unknown", "Do you want to proceed?"),
            Some("confirmation")
        );
        assert_eq!(
            detect_waiting_hint_for_agent("unknown", "Normal output"),
            None
        );
    }
}
