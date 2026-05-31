use portable_pty::ChildKiller;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum BackgroundStatus {
    Running,
    Exited(i32),
    Failed(String),
    Stopped,
}

impl std::fmt::Display for BackgroundStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackgroundStatus::Running => write!(f, "running"),
            BackgroundStatus::Exited(code) => write!(f, "exited({})", code),
            BackgroundStatus::Failed(err) => write!(f, "failed: {}", err),
            BackgroundStatus::Stopped => write!(f, "stopped"),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SessionOutputMode {
    BackgroundLog,
    Foreground,
}

pub type SessionInput = Arc<Mutex<Box<dyn Write + Send>>>;
pub type SessionControl = Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>;

#[allow(dead_code)]
pub struct BackgroundSession {
    pub id: String,
    pub plan: String,
    pub agent: String,
    pub status: BackgroundStatus,
    pub transcript_path: PathBuf,
    pub killer: Option<Arc<Mutex<Box<dyn ChildKiller + Send + Sync>>>>,
    pub input: Option<SessionInput>,
    pub output_mode: Arc<Mutex<SessionOutputMode>>,
    pub control: Option<SessionControl>,
    pub waiting_hint: Arc<Mutex<Option<String>>>,
}

impl std::fmt::Debug for BackgroundSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackgroundSession")
            .field("id", &self.id)
            .field("plan", &self.plan)
            .field("agent", &self.agent)
            .field("status", &self.status)
            .field("transcript_path", &self.transcript_path)
            .field("killer", &self.killer.is_some())
            .field("input", &self.input.is_some())
            .field(
                "output_mode",
                &self
                    .output_mode
                    .lock()
                    .map(|g| *g)
                    .unwrap_or(SessionOutputMode::BackgroundLog),
            )
            .field("control", &self.control.is_some())
            .field(
                "waiting_hint",
                &self.waiting_hint.lock().map(|g| g.clone()).unwrap_or(None),
            )
            .finish()
    }
}

static BACKGROUND_SESSIONS: OnceLock<Mutex<Vec<BackgroundSession>>> = OnceLock::new();

fn get_sessions() -> &'static Mutex<Vec<BackgroundSession>> {
    BACKGROUND_SESSIONS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Helper to generate a stable foreground run ID.
pub fn format_foreground_run_id(agent: &str, timestamp: u64) -> String {
    format!("foreground-{}-{}", agent, timestamp)
}

/// Helper to generate the metadata file content for the foreground session.
pub fn format_foreground_meta(
    agent: &str,
    is_running: bool,
    exit_code: Option<i32>,
    result_path: &str,
) -> String {
    let status_str = if is_running {
        "running".to_string()
    } else {
        format!("exited({})", exit_code.unwrap_or(1))
    };
    format!(
        "agent: {}\nrole: foreground\nstatus: {}\nresult: {}\n",
        agent, status_str, result_path
    )
}

/// Format status output for command mode.
pub fn format_status(
    active_agent: &str,
    foreground_transcript: Option<&str>,
    focused_session_id: Option<&str>,
) -> String {
    let mut output = format!("active: {}\nmode: single-session pty", active_agent);

    if let Some(path) = foreground_transcript {
        output.push_str(&format!("\nforeground transcript: {}", path));
    }

    if let Some(focused_id) = focused_session_id {
        output.push_str(&format!("\nfocused: {}", focused_id));
    }

    let guard = get_sessions().lock().unwrap();
    if !guard.is_empty() {
        output.push_str("\nbackground:");
        for sess in guard.iter() {
            let mode_str = match *sess.output_mode.lock().unwrap() {
                SessionOutputMode::Foreground => "foreground",
                SessionOutputMode::BackgroundLog => "background",
            };
            let waiting_str = if let Ok(hint_guard) = sess.waiting_hint.lock() {
                if let Some(ref hint) = *hint_guard {
                    format!(" waiting({})", hint)
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            };
            output.push_str(&format!(
                "\n  {} {}{} {} {}",
                sess.id,
                sess.status,
                waiting_str,
                mode_str,
                sess.transcript_path.to_string_lossy()
            ));
        }
    }

    output
}

/// Register a new background session in global state.
pub fn register_background_session(session: BackgroundSession) {
    let mut guard = get_sessions().lock().unwrap();
    guard.push(session);
}

/// Mark a background session as exited. Only updates if still Running.
pub fn mark_background_exited(session_id: &str, exit_code: i32) {
    let mut guard = get_sessions().lock().unwrap();
    if let Some(sess) = guard.iter_mut().find(|s| s.id == session_id) {
        if sess.status == BackgroundStatus::Running {
            sess.status = BackgroundStatus::Exited(exit_code);

            if let Ok(mut mode_guard) = sess.output_mode.lock() {
                *mode_guard = SessionOutputMode::BackgroundLog;
            }

            if let Ok(mut hint_guard) = sess.waiting_hint.lock() {
                *hint_guard = None;
            }

            let result_path = format!(".agentmux/runs/{}/result.md", session_id);
            let meta_path = std::path::Path::new(".agentmux")
                .join("runs")
                .join(session_id)
                .join("meta.txt");
            let _ = std::fs::write(
                &meta_path,
                format!(
                    "plan: {}\nagent: {}\nstatus: exited({})\nresult: {}\n",
                    sess.plan, sess.agent, exit_code, result_path
                ),
            );
        }
    }
}

/// Stop a running background session. Returns a user-facing message.
pub fn stop_background_session(session_id: &str) -> String {
    let mut guard = get_sessions().lock().unwrap();

    let Some(sess) = guard.iter_mut().find(|s| s.id == session_id) else {
        return format!("session not found: {}", session_id);
    };

    match sess.status {
        BackgroundStatus::Running => {
            if let Some(killer_arc) = &sess.killer {
                let mut killer_guard = killer_arc.lock().unwrap();
                if killer_guard.kill().is_ok() {
                    sess.status = BackgroundStatus::Stopped;

                    if let Ok(mut hint_guard) = sess.waiting_hint.lock() {
                        *hint_guard = None;
                    }

                    let result_path = format!(".agentmux/runs/{}/result.md", session_id);
                    let meta_path = std::path::Path::new(".agentmux")
                        .join("runs")
                        .join(session_id)
                        .join("meta.txt");
                    let _ = std::fs::write(
                        &meta_path,
                        format!(
                            "plan: {}\nagent: {}\nstatus: stopped\nresult: {}\n",
                            sess.plan, sess.agent, result_path
                        ),
                    );

                    format!("stopped {}", session_id)
                } else {
                    format!("failed to stop {}", session_id)
                }
            } else {
                format!("no process handle for {}", session_id)
            }
        }
        _ => format!("session not running: {}", session_id),
    }
}

/// Stop all currently running background sessions. Returns a user-facing message.
pub fn stop_all_background_sessions() -> String {
    let mut guard = get_sessions().lock().unwrap();
    let mut count = 0;

    for sess in guard.iter_mut() {
        if sess.status == BackgroundStatus::Running {
            if let Some(killer_arc) = &sess.killer {
                let mut killer_guard = killer_arc.lock().unwrap();
                if killer_guard.kill().is_ok() {
                    sess.status = BackgroundStatus::Stopped;

                    if let Ok(mut hint_guard) = sess.waiting_hint.lock() {
                        *hint_guard = None;
                    }

                    let result_path = format!(".agentmux/runs/{}/result.md", sess.id);
                    let meta_path = std::path::Path::new(".agentmux")
                        .join("runs")
                        .join(&sess.id)
                        .join("meta.txt");
                    let _ = std::fs::write(
                        &meta_path,
                        format!(
                            "plan: {}\nagent: {}\nstatus: stopped\nresult: {}\n",
                            sess.plan, sess.agent, result_path
                        ),
                    );
                    count += 1;
                }
            }
        }
    }

    if count > 0 {
        format!("stopped {} background session(s)", count)
    } else {
        "no running background sessions".to_string()
    }
}

/// Retrieve the IDs of all currently running background sessions.
pub fn get_running_session_ids() -> Vec<String> {
    let guard = get_sessions().lock().unwrap();
    guard
        .iter()
        .filter(|s| s.status == BackgroundStatus::Running)
        .map(|s| s.id.clone())
        .collect()
}

/// Helper to format a waiting notification for stderr.
pub fn format_waiting_notification(session_id: &str, hint: &str) -> String {
    format!(
        "agentmux: {} waiting({}); Ctrl-G focus {}",
        session_id, hint, session_id
    )
}

/// Helper to generate a result.md template.
pub fn format_result_template(run_id: &str) -> String {
    format!(
        "# Result: {}\n\nStatus: running\n\n## Summary\n\n## Files Changed\n\n## Verification\n\n## Blockers\n",
        run_id
    )
}

/// Helper to generate the prompt instruction about the result file path.
pub fn format_result_instruction(result_path: &str) -> String {
    format!(
        "When done, write your final result to {} with summary, files changed, verification run, and blockers.",
        result_path
    )
}

/// Set a waiting hint for a running background session.
pub fn set_background_waiting_hint(session_id: &str, hint: &str) -> bool {
    let mut guard = get_sessions().lock().unwrap();
    if let Some(sess) = guard.iter_mut().find(|s| s.id == session_id) {
        let changed = if let Ok(mut hint_guard) = sess.waiting_hint.lock() {
            if let Some(ref current) = *hint_guard {
                if current == hint {
                    false
                } else {
                    *hint_guard = Some(hint.to_string());
                    true
                }
            } else {
                *hint_guard = Some(hint.to_string());
                true
            }
        } else {
            false
        };

        if changed {
            let result_path = format!(".agentmux/runs/{}/result.md", session_id);
            let meta_path = std::path::Path::new(".agentmux")
                .join("runs")
                .join(session_id)
                .join("meta.txt");
            let _ = std::fs::write(
                &meta_path,
                format!(
                    "plan: {}\nagent: {}\nstatus: {}\nwaiting: {}\nresult: {}\n",
                    sess.plan, sess.agent, sess.status, hint, result_path
                ),
            );
        }

        changed
    } else {
        false
    }
}

/// Clear the waiting hint for a background session.
pub fn clear_background_waiting_hint(session_id: &str) {
    let mut guard = get_sessions().lock().unwrap();
    if let Some(sess) = guard.iter_mut().find(|s| s.id == session_id) {
        if let Ok(mut hint_guard) = sess.waiting_hint.lock() {
            *hint_guard = None;
        }

        let result_path = format!(".agentmux/runs/{}/result.md", session_id);
        let meta_path = std::path::Path::new(".agentmux")
            .join("runs")
            .join(session_id)
            .join("meta.txt");
        let _ = std::fs::write(
            &meta_path,
            format!(
                "plan: {}\nagent: {}\nstatus: {}\nwaiting: none\nresult: {}\n",
                sess.plan, sess.agent, sess.status, result_path
            ),
        );
    }
}

/// Focus a running background session. Sets its output mode to Foreground and returns its input handle.
pub fn focus_background_session(session_id: &str) -> Result<SessionInput, String> {
    let guard = get_sessions().lock().unwrap();
    let Some(sess) = guard.iter().find(|s| s.id == session_id) else {
        return Err(format!("session not found: {}", session_id));
    };

    if sess.status != BackgroundStatus::Running {
        return Err(format!("session not running: {}", session_id));
    }

    let Some(input) = &sess.input else {
        return Err(format!("no input handle for session: {}", session_id));
    };

    let mut mode_guard = sess.output_mode.lock().unwrap();
    *mode_guard = SessionOutputMode::Foreground;

    Ok(Arc::clone(input))
}

/// Detach a focused background session. Restores its output mode to BackgroundLog.
pub fn detach_background_session(session_id: &str) -> Result<(), String> {
    let guard = get_sessions().lock().unwrap();
    let Some(sess) = guard.iter().find(|s| s.id == session_id) else {
        return Err(format!("session not found: {}", session_id));
    };

    let mut mode_guard = sess.output_mode.lock().unwrap();
    *mode_guard = SessionOutputMode::BackgroundLog;

    Ok(())
}

/// Check if a background session is running.
pub fn is_background_session_running(session_id: &str) -> bool {
    let guard = get_sessions().lock().unwrap();
    guard
        .iter()
        .find(|s| s.id == session_id)
        .map(|s| s.status == BackgroundStatus::Running)
        .unwrap_or(false)
}

/// Resize all active background PTY sessions.
pub fn resize_background_sessions(size: portable_pty::PtySize) {
    let controls = {
        let guard = get_sessions().lock().unwrap();
        guard
            .iter()
            .filter(|s| s.status == BackgroundStatus::Running)
            .filter_map(|s| s.control.clone())
            .collect::<Vec<_>>()
    };

    for ctrl_arc in controls {
        if let Ok(ctrl) = ctrl_arc.lock() {
            let _ = ctrl.resize(size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::path::PathBuf;

    #[derive(Debug)]
    struct MockChildKiller;
    impl portable_pty::ChildKiller for MockChildKiller {
        fn kill(&mut self) -> std::io::Result<()> {
            Ok(())
        }
        fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
            Box::new(MockChildKiller)
        }
    }

    #[test]
    fn test_background_session_debug_formatting() {
        let dummy_writer: Box<dyn Write + Send> = Box::new(Cursor::new(Vec::new()));
        let sess = BackgroundSession {
            id: "test-session".to_string(),
            plan: "test-plan".to_string(),
            agent: "test-agent".to_string(),
            status: BackgroundStatus::Running,
            transcript_path: PathBuf::from("transcript.ansi"),
            killer: Some(Arc::new(Mutex::new(Box::new(MockChildKiller)))),
            input: Some(Arc::new(Mutex::new(dummy_writer))),
            output_mode: Arc::new(Mutex::new(SessionOutputMode::BackgroundLog)),
            control: None,
            waiting_hint: Arc::new(Mutex::new(None)),
        };

        let debug_str = format!("{:?}", sess);
        assert!(debug_str.contains("id: \"test-session\""));
        assert!(debug_str.contains("input: true"));
        assert!(debug_str.contains("output_mode: BackgroundLog"));
        assert!(debug_str.contains("control: false"));
        assert!(debug_str.contains("waiting_hint: None"));
    }

    #[test]
    fn test_status_formatting_with_new_fields() {
        let active_agent = "agy";
        let status_str_no_transcript = format_status(active_agent, None, None);
        assert!(status_str_no_transcript.contains("active: agy"));
        assert!(status_str_no_transcript.contains("mode: single-session pty"));
        assert!(!status_str_no_transcript.contains("foreground transcript:"));

        let status_str_with_transcript =
            format_status(active_agent, Some("my-path/transcript.ansi"), None);
        assert!(
            status_str_with_transcript.contains("foreground transcript: my-path/transcript.ansi")
        );
    }

    #[test]
    fn test_format_foreground_run_id() {
        let id = format_foreground_run_id("codex", 1777598300);
        assert_eq!(id, "foreground-codex-1777598300");
    }

    #[test]
    fn test_format_foreground_meta() {
        let running_meta = format_foreground_meta("codex", true, None, "my-result-path.md");
        assert!(running_meta.contains("agent: codex"));
        assert!(running_meta.contains("role: foreground"));
        assert!(running_meta.contains("status: running"));
        assert!(running_meta.contains("result: my-result-path.md"));

        let exited_meta = format_foreground_meta("codex", false, Some(0), "my-result-path.md");
        assert!(exited_meta.contains("status: exited(0)"));
        assert!(exited_meta.contains("result: my-result-path.md"));
    }

    #[test]
    fn test_format_result_template() {
        let temp = format_result_template("test-run");
        assert!(temp.contains("# Result: test-run"));
        assert!(temp.contains("Status: running"));
        assert!(temp.contains("## Summary"));
        assert!(temp.contains("## Files Changed"));
    }

    #[test]
    fn test_format_result_instruction() {
        let inst = format_result_instruction("test-path.md");
        assert!(inst.contains("test-path.md"));
        assert!(inst.contains("When done, write your final result to"));
    }

    #[test]
    fn test_stop_with_new_fields() {
        let run_dir = std::path::Path::new(".agentmux")
            .join("runs")
            .join("test-stop-session");
        let _ = std::fs::create_dir_all(&run_dir);

        let dummy_writer: Box<dyn Write + Send> = Box::new(Cursor::new(Vec::new()));
        let sess = BackgroundSession {
            id: "test-stop-session".to_string(),
            plan: "test-plan".to_string(),
            agent: "test-agent".to_string(),
            status: BackgroundStatus::Running,
            transcript_path: PathBuf::from("test-stop-transcript.ansi"),
            killer: Some(Arc::new(Mutex::new(Box::new(MockChildKiller)))),
            input: Some(Arc::new(Mutex::new(dummy_writer))),
            output_mode: Arc::new(Mutex::new(SessionOutputMode::BackgroundLog)),
            control: None,
            waiting_hint: Arc::new(Mutex::new(None)),
        };

        register_background_session(sess);

        let stop_msg = stop_background_session("test-stop-session");
        assert!(stop_msg.contains("stopped test-stop-session"));

        let _ = std::fs::remove_dir_all(run_dir);
    }

    #[test]
    fn test_focus_unknown_session() {
        let result = focus_background_session("unknown-session-id");
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            "session not found: unknown-session-id"
        );
    }

    #[test]
    fn test_detach_unknown_session() {
        let result = detach_background_session("unknown-session-id");
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            "session not found: unknown-session-id"
        );
    }

    #[test]
    fn test_is_background_session_running() {
        assert!(!is_background_session_running("nonexistent-session"));
    }

    #[test]
    fn test_mark_background_exited_resets_mode() {
        let dummy_writer: Box<dyn Write + Send> = Box::new(Cursor::new(Vec::new()));
        let output_mode = Arc::new(Mutex::new(SessionOutputMode::Foreground));
        let sess = BackgroundSession {
            id: "test-exit-session".to_string(),
            plan: "test-plan".to_string(),
            agent: "test-agent".to_string(),
            status: BackgroundStatus::Running,
            transcript_path: PathBuf::from("test-exit-transcript.ansi"),
            killer: Some(Arc::new(Mutex::new(Box::new(MockChildKiller)))),
            input: Some(Arc::new(Mutex::new(dummy_writer))),
            output_mode: Arc::clone(&output_mode),
            control: None,
            waiting_hint: Arc::new(Mutex::new(None)),
        };

        register_background_session(sess);
        assert_eq!(*output_mode.lock().unwrap(), SessionOutputMode::Foreground);

        mark_background_exited("test-exit-session", 0);
        assert_eq!(
            *output_mode.lock().unwrap(),
            SessionOutputMode::BackgroundLog
        );
    }

    #[test]
    fn test_format_status_with_waiting_hint() {
        let dummy_writer: Box<dyn Write + Send> = Box::new(Cursor::new(Vec::new()));
        let sess = BackgroundSession {
            id: "test-wait-session".to_string(),
            plan: "test-plan".to_string(),
            agent: "test-agent".to_string(),
            status: BackgroundStatus::Running,
            transcript_path: PathBuf::from("test-wait-transcript.ansi"),
            killer: Some(Arc::new(Mutex::new(Box::new(MockChildKiller)))),
            input: Some(Arc::new(Mutex::new(dummy_writer))),
            output_mode: Arc::new(Mutex::new(SessionOutputMode::BackgroundLog)),
            control: None,
            waiting_hint: Arc::new(Mutex::new(Some("approval".to_string()))),
        };

        register_background_session(sess);

        let status_str = format_status("codex", None, None);
        assert!(status_str.contains(
            "test-wait-session running waiting(approval) background test-wait-transcript.ansi"
        ));
    }

    #[test]
    fn test_stopping_clears_waiting_state() {
        let dummy_writer: Box<dyn Write + Send> = Box::new(Cursor::new(Vec::new()));
        let sess = BackgroundSession {
            id: "test-stop-clear".to_string(),
            plan: "test-plan".to_string(),
            agent: "test-agent".to_string(),
            status: BackgroundStatus::Running,
            transcript_path: PathBuf::from("test-stop-clear.ansi"),
            killer: Some(Arc::new(Mutex::new(Box::new(MockChildKiller)))),
            input: Some(Arc::new(Mutex::new(dummy_writer))),
            output_mode: Arc::new(Mutex::new(SessionOutputMode::BackgroundLog)),
            control: None,
            waiting_hint: Arc::new(Mutex::new(Some("permission".to_string()))),
        };

        register_background_session(sess);

        // Verify it is initially waiting
        let hint_val = {
            let guard = get_sessions().lock().unwrap();
            let s = guard.iter().find(|x| x.id == "test-stop-clear").unwrap();
            s.waiting_hint.lock().unwrap().clone()
        };
        assert_eq!(hint_val, Some("permission".to_string()));

        // Stop session
        let _ = stop_background_session("test-stop-clear");

        // Verify waiting hint is cleared
        let cleared_val = {
            let guard = get_sessions().lock().unwrap();
            let s = guard.iter().find(|x| x.id == "test-stop-clear").unwrap();
            s.waiting_hint.lock().unwrap().clone()
        };
        assert_eq!(cleared_val, None);
    }

    #[test]
    fn test_format_waiting_notification() {
        let text = format_waiting_notification("my-session", "approval");
        assert_eq!(
            text,
            "agentmux: my-session waiting(approval); Ctrl-G focus my-session"
        );
    }

    #[test]
    fn test_set_background_waiting_hint_behavior() {
        let dummy_writer: Box<dyn Write + Send> = Box::new(Cursor::new(Vec::new()));
        let sess = BackgroundSession {
            id: "test-hint-behavior".to_string(),
            plan: "test-plan".to_string(),
            agent: "test-agent".to_string(),
            status: BackgroundStatus::Running,
            transcript_path: PathBuf::from("test-hint-behavior.ansi"),
            killer: Some(Arc::new(Mutex::new(Box::new(MockChildKiller)))),
            input: Some(Arc::new(Mutex::new(dummy_writer))),
            output_mode: Arc::new(Mutex::new(SessionOutputMode::BackgroundLog)),
            control: None,
            waiting_hint: Arc::new(Mutex::new(None)),
        };

        register_background_session(sess);

        // 1. Initial set -> returns true
        let res1 = set_background_waiting_hint("test-hint-behavior", "approval");
        assert!(res1);

        // 2. Same set repeated -> returns false
        let res2 = set_background_waiting_hint("test-hint-behavior", "approval");
        assert!(!res2);

        // 3. Different set -> returns true
        let res3 = set_background_waiting_hint("test-hint-behavior", "permission");
        assert!(res3);
    }
}
