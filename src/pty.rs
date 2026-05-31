use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{self, IsTerminal, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

/// Set the terminal window title using ANSI OSC escape sequence.
fn set_terminal_title(title: &str) {
    if io::stderr().is_terminal() {
        let mut stderr = io::stderr();
        // OSC 0 ; title BEL
        let _ = write!(stderr, "\x1b]0;{}\x07", title);
        let _ = stderr.flush();
    }
}

pub enum SessionAction {
    Exit(i32),
    SwitchAgent { agent: String, prompt: String },
}

struct RawModeGuard {
    active: bool,
}

impl RawModeGuard {
    fn new() -> io::Result<Self> {
        let active = io::stdin().is_terminal();
        if active {
            crossterm::terminal::enable_raw_mode()?;
        }
        Ok(RawModeGuard { active })
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
}

fn current_pty_size() -> PtySize {
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn spawn_background_pty(plan: &str, agent_key: &str) -> Result<(), String> {
    let launch = crate::config::load_agent_launch(agent_key)?;

    // 1. Generate run-id and directories
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let run_id = format!("{}-{}-{}", plan, agent_key, timestamp);
    let run_dir = std::path::Path::new(".agentmux").join("runs").join(&run_id);
    std::fs::create_dir_all(&run_dir).map_err(|e| e.to_string())?;

    let result_path = run_dir.join("result.md");
    let result_path_str = result_path.to_string_lossy().into_owned();

    // Create result.md template without overwriting an existing result.
    if !result_path.exists() {
        let template_content = crate::sessions::format_result_template(&run_id);
        std::fs::write(&result_path, template_content).map_err(|e| e.to_string())?;
    }

    // 2. Write metadata
    let meta_path = run_dir.join("meta.txt");
    std::fs::write(
        &meta_path,
        format!("plan: {plan}\nagent: {agent_key}\nstatus: running\nresult: {result_path_str}\n"),
    )
    .map_err(|e| e.to_string())?;

    let transcript_path = run_dir.join("transcript.ansi");

    // 3. Open PTY
    let pty_system = native_pty_system();
    let size = current_pty_size();
    let pair = pty_system.openpty(size).map_err(|e| e.to_string())?;

    // 4. Build command
    let mut cmd = CommandBuilder::new(&launch.command);
    cmd.args(&launch.args);

    if let Ok(cwd) = std::env::current_dir() {
        cmd.cwd(cwd);
    }
    for (key, val) in std::env::vars() {
        cmd.env(key, val);
    }

    // 5. Spawn child
    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| {
            let is_not_found = if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                io_err.kind() == std::io::ErrorKind::NotFound
            } else {
                let err_str = e.to_string().to_lowercase();
                err_str.contains("no such file")
                    || err_str.contains("not found")
                    || err_str.contains("find the file specified")
            };
            if is_not_found {
                format!(
                    "could not launch agent `{}` command `{}`; is it installed and configured correctly?",
                    agent_key, launch.command
                )
            } else {
                format!("failed to spawn command `{}`: {}", launch.command, e)
            }
        })?;

    drop(pair.slave);

    let mut pty_reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let pty_writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let master_control: crate::sessions::SessionControl = Arc::new(Mutex::new(pair.master));

    // Wrap PTY writer in Arc<Mutex<Box<dyn Write + Send>>>
    let pty_writer_box: Box<dyn Write + Send> = Box::new(pty_writer);
    let pty_writer_shared = Arc::new(Mutex::new(pty_writer_box));

    // 6. Write startup prompt using the retained writer
    let prompt_inst = crate::sessions::format_result_instruction(&result_path_str);
    let prompt = format!(
        "Read .agentmux/plans/{plan}/ and implement the task described there. Follow acceptance.md and constraints.md. {prompt_inst}\n"
    );
    {
        let mut writer_guard = pty_writer_shared.lock().unwrap();
        let _ = writer_guard.write_all(prompt.as_bytes());
        let _ = writer_guard.flush();
    }

    // Clone child killer handle
    let killer = child.clone_killer();
    let killer_shared = Arc::new(Mutex::new(killer));

    // Create shared output mode
    let output_mode = Arc::new(Mutex::new(
        crate::sessions::SessionOutputMode::BackgroundLog,
    ));
    let output_mode_clone = Arc::clone(&output_mode);

    // 7. Register in global session state
    let session = crate::sessions::BackgroundSession {
        id: run_id.clone(),
        plan: plan.to_string(),
        agent: agent_key.to_string(),
        status: crate::sessions::BackgroundStatus::Running,
        transcript_path: transcript_path.clone(),
        killer: Some(killer_shared),
        input: Some(Arc::clone(&pty_writer_shared)),
        output_mode: Arc::clone(&output_mode),
        control: Some(master_control),
        waiting_hint: Arc::new(Mutex::new(None)),
    };
    crate::sessions::register_background_session(session);

    // 8. Spawn thread to copy PTY output to transcript file and wait for child on completion
    let run_id_clone = run_id.clone();
    let agent_name = agent_key.to_string();
    thread::spawn(move || {
        if let Ok(mut file) = std::fs::File::create(&transcript_path) {
            let mut buffer = [0u8; 4096];
            let mut rolling_buf = String::new();
            while let Ok(n) = pty_reader.read(&mut buffer) {
                if n == 0 {
                    break;
                }
                // Write transcript
                if file.write_all(&buffer[..n]).is_err() || file.flush().is_err() {
                    break;
                }

                // Maintain rolling buffer for waiting hint detection
                let decoded = String::from_utf8_lossy(&buffer[..n]);
                rolling_buf.push_str(&decoded);
                if rolling_buf.len() > 4096 {
                    let keep_idx = rolling_buf.len() - 4096;
                    if let Some((valid_idx, _)) =
                        rolling_buf.char_indices().find(|&(idx, _)| idx >= keep_idx)
                    {
                        rolling_buf = rolling_buf[valid_idx..].to_string();
                    } else {
                        rolling_buf.clear();
                    }
                }

                if let Some(hint) =
                    crate::waiting::detect_waiting_hint_for_agent(&agent_name, &rolling_buf)
                {
                    if crate::sessions::set_background_waiting_hint(&run_id_clone, hint) {
                        let is_focused = {
                            if let Ok(guard) = output_mode_clone.lock() {
                                *guard == crate::sessions::SessionOutputMode::Foreground
                            } else {
                                false
                            }
                        };
                        if !is_focused {
                            let notification =
                                crate::sessions::format_waiting_notification(&run_id_clone, hint);
                            let mut stderr = io::stderr();
                            let _ = stderr.write_all(notification.as_bytes());
                            let _ = stderr.write_all(b"\r\n");
                            let _ = stderr.flush();
                        }
                    }
                }

                // Check output mode for mirroring to stdout
                let current_mode = {
                    if let Ok(guard) = output_mode_clone.lock() {
                        *guard
                    } else {
                        crate::sessions::SessionOutputMode::BackgroundLog
                    }
                };

                match current_mode {
                    crate::sessions::SessionOutputMode::BackgroundLog => {}
                    crate::sessions::SessionOutputMode::Foreground => {
                        let mut stdout = io::stdout();
                        let _ = stdout.write_all(&buffer[..n]);
                        let _ = stdout.flush();
                    }
                }
            }
        }

        // Wait for child to exit and update status
        let exit_code = match child.wait() {
            Ok(status) => status.exit_code() as i32,
            Err(_) => 1,
        };

        crate::sessions::mark_background_exited(&run_id_clone, exit_code);
    });

    Ok(())
}

/// Write a message to stderr with \r\n line ending.
fn write_stderr(stderr: &mut io::Stderr, msg: &str) {
    let formatted = msg.replace('\n', "\r\n");
    let _ = stderr.write_all(formatted.as_bytes());
    let _ = stderr.write_all(b"\r\n");
    let _ = stderr.flush();
}

fn execute_wrapper_command(
    result: crate::commands::WrapperCommandResult,
    active_input: &Arc<Mutex<crate::sessions::SessionInput>>,
    foreground_input: &crate::sessions::SessionInput,
    focused_session_id: &mut Option<String>,
    foreground_output_visible: &std::sync::atomic::AtomicBool,
    fg_transcript_path: Option<&str>,
    active_agent: &str,
) -> Option<SessionAction> {
    let mut stderr = io::stderr();
    match result {
        crate::commands::WrapperCommandResult::Help => {
            write_stderr(
                &mut stderr,
                "agentmux commands: help, status, send, stop, focus, detach, runs, result, tail, stop-all",
            );
            None
        }
        crate::commands::WrapperCommandResult::Status => {
            let status_text = crate::sessions::format_status(
                active_agent,
                fg_transcript_path,
                focused_session_id.as_deref(),
            );
            write_stderr(&mut stderr, &status_text);
            None
        }
        crate::commands::WrapperCommandResult::Print(msg) => {
            write_stderr(&mut stderr, &msg);
            None
        }
        crate::commands::WrapperCommandResult::Send {
            plan,
            agent,
            background,
        } => {
            if background {
                match spawn_background_pty(&plan, &agent) {
                    Ok(_) => {
                        let msg = format!("sent plan `{}` to `{}` in background", plan, agent);
                        write_stderr(&mut stderr, &msg);
                    }
                    Err(err) => {
                        let msg = format!("failed to spawn background session: {}", err);
                        write_stderr(&mut stderr, &msg);
                    }
                }
                return None;
            } else {
                let prompt = format!(
                    "Read .agentmux/plans/{plan}/ and implement the task described there. Follow acceptance.md and constraints.md. When done, summarize files changed, verification run, and any blockers."
                );
                if let Some(focused_id) = focused_session_id.take() {
                    let _ = crate::sessions::detach_background_session(&focused_id);
                    *active_input.lock().unwrap() = Arc::clone(foreground_input);
                }
                foreground_output_visible.store(true, std::sync::atomic::Ordering::Relaxed);
                return Some(SessionAction::SwitchAgent { agent, prompt });
            }
        }
        crate::commands::WrapperCommandResult::Stop { session_id } => {
            let msg = crate::sessions::stop_background_session(&session_id);
            write_stderr(&mut stderr, &msg);
            if Some(&session_id) == focused_session_id.as_ref() {
                let _ = crate::sessions::detach_background_session(&session_id);
                *focused_session_id = None;
                *active_input.lock().unwrap() = Arc::clone(foreground_input);
                foreground_output_visible.store(true, std::sync::atomic::Ordering::Relaxed);
                write_stderr(&mut stderr, "session stopped; detached");
            }
            return None;
        }
        crate::commands::WrapperCommandResult::Focus { session_id } => {
            if let Some(old_id) = focused_session_id.as_ref() {
                let _ = crate::sessions::detach_background_session(old_id);
            }
            match crate::sessions::focus_background_session(&session_id) {
                Ok(session_input) => {
                    *focused_session_id = Some(session_id.clone());
                    *active_input.lock().unwrap() = session_input;
                    foreground_output_visible.store(false, std::sync::atomic::Ordering::Relaxed);
                    write_stderr(&mut stderr, &format!("focused {}", session_id));
                }
                Err(err) => {
                    write_stderr(&mut stderr, &format!("failed to focus: {}", err));
                }
            }
            return None;
        }
        crate::commands::WrapperCommandResult::Detach => {
            if let Some(focused_id) = focused_session_id.take() {
                let _ = crate::sessions::detach_background_session(&focused_id);
                *active_input.lock().unwrap() = Arc::clone(foreground_input);
                foreground_output_visible.store(true, std::sync::atomic::Ordering::Relaxed);
                write_stderr(&mut stderr, &format!("detached {}", focused_id));
            } else {
                write_stderr(&mut stderr, "no background session is focused");
            }
            return None;
        }
        crate::commands::WrapperCommandResult::Runs => {
            let runs_text = crate::runs::format_runs_list();
            write_stderr(&mut stderr, &runs_text);
            return None;
        }
        crate::commands::WrapperCommandResult::ResultCmd { run_id } => {
            let result_text = crate::runs::read_run_result(&run_id);
            write_stderr(&mut stderr, &result_text);
            return None;
        }
        crate::commands::WrapperCommandResult::Tail { run_id, lines } => {
            let tail_text = crate::runs::read_run_transcript_tail(&run_id, lines);
            write_stderr(&mut stderr, &tail_text);
            return None;
        }
        crate::commands::WrapperCommandResult::StopAll => {
            let msg = crate::sessions::stop_all_background_sessions();
            write_stderr(&mut stderr, &msg);
            if let Some(focused_id) = focused_session_id.as_ref() {
                if !crate::sessions::is_background_session_running(focused_id) {
                    let _ = crate::sessions::detach_background_session(focused_id);
                    *focused_session_id = None;
                    *active_input.lock().unwrap() = Arc::clone(foreground_input);
                    foreground_output_visible.store(true, std::sync::atomic::Ordering::Relaxed);
                    write_stderr(&mut stderr, "session stopped; detached");
                }
            }
            return None;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputProcessResult {
    Forward(Vec<u8>),
    EnterInlineCmd,
    None,
}

pub fn process_input_byte(
    b: u8,
    line_start: &mut bool,
    prefix_bytes: &[u8],
    prefix_buffer: &mut Vec<u8>,
) -> InputProcessResult {
    if *line_start {
        let match_len = prefix_buffer.len();
        if match_len < prefix_bytes.len() && b == prefix_bytes[match_len] {
            prefix_buffer.push(b);
            if prefix_buffer.len() == prefix_bytes.len() {
                prefix_buffer.clear();
                return InputProcessResult::EnterInlineCmd;
            }
            return InputProcessResult::None;
        } else {
            let mut out = Vec::new();
            if !prefix_buffer.is_empty() {
                out.extend_from_slice(prefix_buffer);
                prefix_buffer.clear();
            }
            out.push(b);
            if b == b'\n' || b == b'\r' {
                *line_start = true;
            } else {
                *line_start = false;
            }
            return InputProcessResult::Forward(out);
        }
    }

    if b == b'\n' || b == b'\r' {
        *line_start = true;
    } else {
        *line_start = false;
    }
    InputProcessResult::Forward(vec![b])
}

fn handle_command_mode(
    stdin: &mut io::Stdin,
    active_input: &Arc<Mutex<crate::sessions::SessionInput>>,
    foreground_input: &crate::sessions::SessionInput,
    focused_session_id: &mut Option<String>,
    foreground_output_visible: &std::sync::atomic::AtomicBool,
    fg_transcript_path: Option<&str>,
    active_agent: &str,
) -> Option<SessionAction> {
    let mut stderr = io::stderr();
    let _ = stderr.write_all(b"agentmux> ");
    let _ = stderr.flush();

    let mut buffer = String::new();
    let mut byte = [0u8; 1];

    while let Ok(n) = stdin.read(&mut byte) {
        if n == 0 {
            break;
        }
        let b = byte[0];
        match b {
            0x03 | 0x1b => {
                // Ctrl-C or Esc
                let _ = stderr.write_all(b"\r\n");
                let _ = stderr.flush();
                return None;
            }
            0x0d | 0x0a => {
                // Enter
                let _ = stderr.write_all(b"\r\n");
                let _ = stderr.flush();
                let cmd = buffer.trim();
                if !cmd.is_empty() {
                    let result = crate::commands::run_wrapper_command(cmd);
                    if let Some(action) = execute_wrapper_command(
                        result,
                        active_input,
                        foreground_input,
                        focused_session_id,
                        foreground_output_visible,
                        fg_transcript_path,
                        active_agent,
                    ) {
                        return Some(action);
                    }
                }
                return None;
            }
            0x08 | 0x7f => {
                // Backspace or Del
                if !buffer.is_empty() {
                    buffer.pop();
                    let _ = stderr.write_all(b"\x08 \x08");
                    let _ = stderr.flush();
                }
            }
            other => {
                if other.is_ascii() && !other.is_ascii_control() {
                    if let Some(c) = char::from_u32(other as u32) {
                        buffer.push(c);
                        let _ = stderr.write_all(&[other]);
                        let _ = stderr.flush();
                    }
                }
            }
        }
    }
    None
}

pub fn run_pty_command(
    agent_key: &str,
    args: Vec<String>,
    startup_prompt: Option<String>,
) -> Result<SessionAction, String> {
    let launch = crate::config::load_agent_launch(agent_key)?;

    let pty_system = native_pty_system();

    let size = current_pty_size();

    let pair = pty_system
        .openpty(size)
        .map_err(|e| format!("failed to open pty: {}", e))?;

    let mut cmd = CommandBuilder::new(&launch.command);
    cmd.args(&launch.args);
    cmd.args(args);

    if let Ok(cwd) = std::env::current_dir() {
        cmd.cwd(cwd);
    }

    for (key, val) in std::env::vars() {
        cmd.env(key, val);
    }

    let mut child = pair.slave.spawn_command(cmd).map_err(|e| {
        let is_not_found = if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
            io_err.kind() == std::io::ErrorKind::NotFound
        } else {
            let err_str = e.to_string().to_lowercase();
            err_str.contains("no such file")
                || err_str.contains("not found")
                || err_str.contains("find the file specified")
        };
        if is_not_found {
            format!(
                "could not launch agent `{}` command `{}`; is it installed and configured correctly?",
                agent_key, launch.command
            )
        } else {
            format!("failed to spawn command `{}`: {}", launch.command, e)
        }
    })?;

    // Set terminal title to show active agent
    set_terminal_title(&format!("agentmux: {}", agent_key));

    // Clone killer before moving child into wait thread
    let mut child_killer = child.clone_killer();

    // Drop the slave end in the parent process to prevent reader deadlock on EOF
    drop(pair.slave);

    let mut pty_reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("failed to clone pty reader: {}", e))?;
    let mut pty_writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("failed to take pty writer: {}", e))?;

    let master_control = Arc::new(Mutex::new(pair.master));

    // Create a foreground run directory and transcript file
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let run_id = crate::sessions::format_foreground_run_id(agent_key, timestamp);
    let run_dir = std::path::Path::new(".agentmux").join("runs").join(&run_id);

    let result_path = run_dir.join("result.md");
    let result_path_str = result_path.to_string_lossy().into_owned();

    let mut transcript_file = None;
    let mut fg_transcript_path = None;
    if std::fs::create_dir_all(&run_dir).is_ok() {
        // Create result.md template without overwriting an existing result.
        if !result_path.exists() {
            let template_content = crate::sessions::format_result_template(&run_id);
            let _ = std::fs::write(&result_path, template_content);
        }

        let meta_path = run_dir.join("meta.txt");
        let _ = std::fs::write(
            &meta_path,
            crate::sessions::format_foreground_meta(agent_key, true, None, &result_path_str),
        );
        let transcript_path = run_dir.join("transcript.ansi");
        if let Ok(file) = std::fs::File::create(&transcript_path) {
            transcript_file = Some(file);
            fg_transcript_path = Some(transcript_path.to_string_lossy().into_owned());
        }
    }

    // Inject startup prompt if present
    if let Some(mut prompt) = startup_prompt {
        let target_phrase =
            "When done, summarize files changed, verification run, and any blockers.";
        let instruction = crate::sessions::format_result_instruction(&result_path_str);
        if prompt.contains(target_phrase) {
            prompt = prompt.replace(target_phrase, &instruction);
        } else {
            prompt.push_str(" ");
            prompt.push_str(&instruction);
        }

        let mut temp_writer = pty_writer;
        let _ = temp_writer.write_all(prompt.as_bytes());
        let _ = temp_writer.write_all(b"\n");
        let _ = temp_writer.flush();
        pty_writer = temp_writer;
    }

    let foreground_input: crate::sessions::SessionInput =
        Arc::new(Mutex::new(Box::new(pty_writer)));
    let active_input = Arc::new(Mutex::new(Arc::clone(&foreground_input)));

    // Create a channel to communicate the session action back to the main thread
    let (tx, rx) = std::sync::mpsc::channel::<SessionAction>();

    use std::sync::atomic::{AtomicBool, Ordering};
    let foreground_output_visible = Arc::new(AtomicBool::new(true));

    // Thread for copying PTY output to stdout and transcript
    let foreground_output_visible_clone = Arc::clone(&foreground_output_visible);
    let mut transcript_file_clone = transcript_file;
    let _output_thread = thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        let mut stdout = io::stdout();
        while let Ok(n) = pty_reader.read(&mut buffer) {
            if n == 0 {
                break;
            }
            if let Some(ref mut file) = transcript_file_clone {
                let _ = file.write_all(&buffer[..n]);
                let _ = file.flush();
            }
            if foreground_output_visible_clone.load(Ordering::Relaxed) {
                if stdout.write_all(&buffer[..n]).is_err() || stdout.flush().is_err() {
                    break;
                }
            }
        }
    });

    let stop_resize = Arc::new(AtomicBool::new(false));
    let stop_resize_clone = Arc::clone(&stop_resize);
    let master_control_clone = Arc::clone(&master_control);

    let _resize_thread = thread::spawn(move || {
        let mut last_size = current_pty_size();
        while !stop_resize_clone.load(Ordering::Relaxed) {
            thread::sleep(std::time::Duration::from_millis(500));
            if stop_resize_clone.load(Ordering::Relaxed) {
                break;
            }
            let size = current_pty_size();
            if size.cols != last_size.cols || size.rows != last_size.rows {
                if let Ok(ctrl) = master_control_clone.lock() {
                    let _ = ctrl.resize(size);
                }
                crate::sessions::resize_background_sessions(size);
                last_size = size;
            }
        }
    });

    // Thread for copying stdin to PTY input, with Ctrl-G interception
    let tx_clone = tx.clone();
    let active_agent = agent_key.to_string();
    let foreground_input_clone = Arc::clone(&foreground_input);
    let active_input_clone = Arc::clone(&active_input);
    let foreground_output_visible_clone2 = Arc::clone(&foreground_output_visible);
    let fg_transcript_str = fg_transcript_path.clone();
    let _input_thread = thread::spawn(move || {
        let mut byte = [0u8; 1];
        let mut stdin = io::stdin();
        let mut focused_session_id: Option<String> = None;
        let prefix = crate::config::load_command_prefix().unwrap_or_else(|_| "//".to_string());
        let prefix_bytes = prefix.as_bytes();

        let mut line_start = true;
        let mut prefix_buffer = Vec::new();
        let mut inline_cmd_mode = false;
        let mut inline_cmd_buffer = String::new();

        while let Ok(n) = stdin.read(&mut byte) {
            if n == 0 {
                break;
            }
            let b = byte[0];

            if inline_cmd_mode {
                match b {
                    0x03 | 0x1b => {
                        inline_cmd_mode = false;
                        inline_cmd_buffer.clear();
                        let mut stderr = io::stderr();
                        let _ = stderr.write_all(b"\r\n");
                        let _ = stderr.flush();
                        line_start = true;
                    }
                    0x0d | 0x0a => {
                        let mut stderr = io::stderr();
                        let _ = stderr.write_all(b"\r\n");
                        let _ = stderr.flush();

                        let cmd = inline_cmd_buffer.trim();
                        if !cmd.is_empty() {
                            let result = crate::commands::run_wrapper_command(cmd);
                            if let Some(action) = execute_wrapper_command(
                                result,
                                &active_input_clone,
                                &foreground_input_clone,
                                &mut focused_session_id,
                                &foreground_output_visible_clone2,
                                fg_transcript_str.as_deref(),
                                &active_agent,
                            ) {
                                let _ = tx_clone.send(action);
                                break;
                            }
                        }

                        inline_cmd_mode = false;
                        inline_cmd_buffer.clear();
                        line_start = true;
                    }
                    0x08 | 0x7f => {
                        if !inline_cmd_buffer.is_empty() {
                            inline_cmd_buffer.pop();
                            let mut stderr = io::stderr();
                            let _ = stderr.write_all(b"\x08 \x08");
                            let _ = stderr.flush();
                        }
                    }
                    other => {
                        if other.is_ascii() && !other.is_ascii_control() {
                            if let Some(c) = char::from_u32(other as u32) {
                                inline_cmd_buffer.push(c);
                                let mut stderr = io::stderr();
                                let _ = stderr.write_all(&[other]);
                                let _ = stderr.flush();
                            }
                        }
                    }
                }
                continue;
            }

            if b == 0x07 {
                // Ctrl-G
                if !prefix_buffer.is_empty() {
                    let current_writer = {
                        let guard = active_input_clone.lock().unwrap();
                        Arc::clone(&*guard)
                    };
                    let mut writer_guard = current_writer.lock().unwrap();
                    let _ = writer_guard.write_all(&prefix_buffer);
                    let _ = writer_guard.flush();
                    prefix_buffer.clear();
                }

                if let Some(action) = handle_command_mode(
                    &mut stdin,
                    &active_input_clone,
                    &foreground_input_clone,
                    &mut focused_session_id,
                    &foreground_output_visible_clone2,
                    fg_transcript_str.as_deref(),
                    &active_agent,
                ) {
                    let _ = tx_clone.send(action);
                    break;
                }
                line_start = true;
                continue;
            }

            // Check focus lifecycle if focused_session_id is set
            let is_running = if let Some(ref session_id) = focused_session_id {
                crate::sessions::is_background_session_running(session_id)
            } else {
                false
            };

            if focused_session_id.is_some() && !is_running {
                let mut stderr = io::stderr();
                let _ = stderr.write_all(b"\r\nfocused session ended; detached\r\n");
                let _ = stderr.flush();

                focused_session_id = None;
                *active_input_clone.lock().unwrap() = Arc::clone(&foreground_input_clone);
                foreground_output_visible_clone2.store(true, Ordering::Relaxed);
            }

            if let Some(ref session_id) = focused_session_id {
                crate::sessions::clear_background_waiting_hint(session_id);
            }

            match process_input_byte(b, &mut line_start, prefix_bytes, &mut prefix_buffer) {
                InputProcessResult::EnterInlineCmd => {
                    inline_cmd_mode = true;
                    inline_cmd_buffer.clear();
                    let mut stderr = io::stderr();
                    let _ = stderr.write_all(b"agentmux> ");
                    let _ = stderr.flush();
                }
                InputProcessResult::Forward(bytes) => {
                    let current_writer = {
                        let guard = active_input_clone.lock().unwrap();
                        Arc::clone(&*guard)
                    };
                    let mut writer_guard = current_writer.lock().unwrap();
                    if writer_guard.write_all(&bytes).is_err() || writer_guard.flush().is_err() {
                        break;
                    }
                }
                InputProcessResult::None => {}
            }
        }
    });

    // Wait thread owns child and calls wait()
    let tx_exit = tx.clone();
    let cmd_name = agent_key.to_string();
    let run_id_clone = run_id.clone();
    let result_path_str_clone = result_path_str.clone();
    let _wait_thread = thread::spawn(move || {
        let code = match child.wait() {
            Ok(status) => status.exit_code() as i32,
            Err(_) => 1,
        };
        // Update meta.txt
        let meta_path = std::path::Path::new(".agentmux")
            .join("runs")
            .join(&run_id_clone)
            .join("meta.txt");
        let _ = std::fs::write(
            &meta_path,
            crate::sessions::format_foreground_meta(
                &cmd_name,
                false,
                Some(code),
                &result_path_str_clone,
            ),
        );
        let _ = tx_exit.send(SessionAction::Exit(code));
    });

    // Configure the parent terminal in raw mode during child lifecycle
    let _raw_mode = RawModeGuard::new().ok();

    // Block on receiving the session action from threads
    let action = rx.recv().unwrap_or(SessionAction::Exit(1));

    stop_resize.store(true, Ordering::Relaxed);
    foreground_output_visible.store(true, Ordering::Relaxed);

    // If switching agents, use cloned killer (not mutex-wrapped child)
    if let SessionAction::SwitchAgent { .. } = &action {
        let _ = child_killer.kill();
    }

    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_prefix_status() {
        let prefix = b"//";
        let mut line_start = true;
        let mut prefix_buffer = Vec::new();

        let res1 = process_input_byte(b'/', &mut line_start, prefix, &mut prefix_buffer);
        assert_eq!(res1, InputProcessResult::None);
        assert_eq!(prefix_buffer, vec![b'/']);
        assert!(line_start);

        let res2 = process_input_byte(b'/', &mut line_start, prefix, &mut prefix_buffer);
        assert_eq!(res2, InputProcessResult::EnterInlineCmd);
        assert!(prefix_buffer.is_empty());
        assert!(line_start);
    }

    #[test]
    fn test_inline_prefix_pass_through_single_slash() {
        let prefix = b"//";
        let mut line_start = true;
        let mut prefix_buffer = Vec::new();

        let res1 = process_input_byte(b'/', &mut line_start, prefix, &mut prefix_buffer);
        assert_eq!(res1, InputProcessResult::None);

        let res2 = process_input_byte(b's', &mut line_start, prefix, &mut prefix_buffer);
        assert_eq!(res2, InputProcessResult::Forward(vec![b'/', b's']));
        assert!(prefix_buffer.is_empty());
        assert!(!line_start);

        let res3 = process_input_byte(b'k', &mut line_start, prefix, &mut prefix_buffer);
        assert_eq!(res3, InputProcessResult::Forward(vec![b'k']));
        assert!(!line_start);
    }

    #[test]
    fn test_inline_prefix_not_at_line_start() {
        let prefix = b"//";
        let mut line_start = true;
        let mut prefix_buffer = Vec::new();

        let res1 = process_input_byte(b'h', &mut line_start, prefix, &mut prefix_buffer);
        assert_eq!(res1, InputProcessResult::Forward(vec![b'h']));
        assert!(!line_start);

        let res2 = process_input_byte(b'/', &mut line_start, prefix, &mut prefix_buffer);
        assert_eq!(res2, InputProcessResult::Forward(vec![b'/']));
        assert!(prefix_buffer.is_empty());
    }
}
