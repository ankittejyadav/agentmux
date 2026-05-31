use std::fs;
use std::path::Path;

pub fn format_runs_list() -> String {
    let runs_dir = Path::new(".agentmux").join("runs");
    format_runs_list_from(&runs_dir)
}

fn format_runs_list_from(runs_dir: &Path) -> String {
    if !runs_dir.exists() || !runs_dir.is_dir() {
        return "no runs found".to_string();
    }

    let mut entries = Vec::new();
    if let Ok(rd) = fs::read_dir(&runs_dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let run_id = entry.file_name().to_string_lossy().into_owned();
                let modified = entry.metadata().and_then(|m| m.modified()).ok();
                entries.push((run_id, path, modified));
            }
        }
    }

    if entries.is_empty() {
        return "no runs found".to_string();
    }

    // Sort newest first by directory modified time
    entries.sort_by(|a, b| match (a.2, b.2) {
        (Some(ta), Some(tb)) => tb.cmp(&ta),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.0.cmp(&b.0),
    });

    let mut output = "runs:".to_string();
    let count = entries.len().min(20);

    for (run_id, path, _) in entries.into_iter().take(count) {
        let mut agent = "unknown".to_string();
        let mut status = "unknown".to_string();
        let mut result = "unknown".to_string();

        let meta_path = path.join("meta.txt");
        if meta_path.exists() {
            if let Ok(content) = fs::read_to_string(&meta_path) {
                for line in content.lines() {
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let key = parts[0].trim();
                        let val = parts[1].trim();
                        match key {
                            "agent" => agent = val.to_string(),
                            "status" => status = val.to_string(),
                            "result" => result = val.to_string(),
                            _ => {}
                        }
                    }
                }
            }
        }

        let transcript_path = path.join("transcript.ansi");
        let transcript = if transcript_path.exists() {
            transcript_path.to_string_lossy().into_owned()
        } else {
            "unknown".to_string()
        };

        output.push_str(&format!(
            "\n  {} {} {} result={} transcript={}",
            run_id, status, agent, result, transcript
        ));
    }

    output
}

pub fn read_run_result(run_id: &str) -> String {
    let runs_dir = Path::new(".agentmux").join("runs");
    read_run_result_from(&runs_dir, run_id)
}

fn read_run_result_from(runs_dir: &Path, run_id: &str) -> String {
    let run_dir = runs_dir.join(run_id);
    if !run_dir.exists() || !run_dir.is_dir() {
        return format!("run not found: {}", run_id);
    }

    let result_path = run_dir.join("result.md");
    if !result_path.exists() || !result_path.is_file() {
        return format!("result not found: {}", run_id);
    }

    match fs::read_to_string(&result_path) {
        Ok(content) => {
            let limit = 64 * 1024; // 64 KiB
            if content.len() > limit {
                let mut truncated = content.chars().take(limit).collect::<String>();
                truncated.push_str("\n[truncated]");
                truncated
            } else {
                content
            }
        }
        Err(err) => format!("failed to read result: {}", err),
    }
}

pub fn read_run_transcript_tail(run_id: &str, lines: usize) -> String {
    let runs_dir = Path::new(".agentmux").join("runs");
    read_run_transcript_tail_from(&runs_dir, run_id, lines)
}

fn read_run_transcript_tail_from(runs_dir: &Path, run_id: &str, lines: usize) -> String {
    let run_dir = runs_dir.join(run_id);
    if !run_dir.exists() || !run_dir.is_dir() {
        return format!("run not found: {}", run_id);
    }

    let transcript_path = run_dir.join("transcript.ansi");
    if !transcript_path.exists() || !transcript_path.is_file() {
        return format!("transcript not found: {}", run_id);
    }

    let capped_lines = lines.min(500);

    match fs::File::open(&transcript_path) {
        Ok(file) => {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(file);
            let mut line_buffer = std::collections::VecDeque::new();

            for line_res in reader.split(b'\n') {
                match line_res {
                    Ok(line_bytes) => {
                        if line_buffer.len() >= capped_lines {
                            line_buffer.pop_front();
                        }
                        line_buffer.push_back(line_bytes);
                    }
                    Err(_) => {
                        break;
                    }
                }
            }

            let mut result_lines = Vec::new();
            for line_bytes in line_buffer {
                let s = String::from_utf8_lossy(&line_bytes).into_owned();
                result_lines.push(s);
            }

            let header = format!("transcript tail: {} last {} lines", run_id, capped_lines);
            format!("{}\n{}", header, result_lines.join("\n"))
        }
        Err(err) => format!("failed to read transcript: {}", err),
    }
}

pub fn cleanup_runs(args: &[String]) -> Result<(), String> {
    let runs_dir = Path::new(".agentmux").join("runs");
    cleanup_runs_from(&runs_dir, args)
}

fn run_meta_status(run_dir: &Path) -> Option<String> {
    let meta_path = run_dir.join("meta.txt");
    let content = fs::read_to_string(meta_path).ok()?;
    for line in content.lines() {
        if let Some((key, value)) = line.split_once(':') {
            if key.trim() == "status" {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn cleanup_runs_from(runs_dir: &Path, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage: agentmux cleanup-runs --dry-run|--older-than <days>".to_string());
    }

    let mut dry_run = false;
    let mut older_than_days: Option<u64> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--older-than" => {
                if i + 1 >= args.len() {
                    return Err(
                        "usage: agentmux cleanup-runs --dry-run|--older-than <days>".to_string()
                    );
                }
                let days_str = &args[i + 1];
                match days_str.parse::<u64>() {
                    Ok(val) => {
                        if val == 0 {
                            return Err(
                                "usage: agentmux cleanup-runs --dry-run|--older-than <days>"
                                    .to_string(),
                            );
                        }
                        older_than_days = Some(val);
                    }
                    Err(_) => {
                        return Err("usage: agentmux cleanup-runs --dry-run|--older-than <days>"
                            .to_string());
                    }
                }
                i += 2;
            }
            _ => {
                return Err(
                    "usage: agentmux cleanup-runs --dry-run|--older-than <days>".to_string()
                );
            }
        }
    }

    if !dry_run && older_than_days.is_none() {
        return Err("usage: agentmux cleanup-runs --dry-run|--older-than <days>".to_string());
    }

    if !runs_dir.exists() || !runs_dir.is_dir() {
        println!("no runs directory found");
        return Ok(());
    }

    let running_sessions = crate::sessions::get_running_session_ids();

    let rd = fs::read_dir(runs_dir).map_err(|e| format!("failed to read runs directory: {}", e))?;
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let run_id = entry.file_name().to_string_lossy().into_owned();

            // Safety: never delete running sessions from in-memory state
            if running_sessions.contains(&run_id) {
                continue;
            }

            if run_meta_status(&path).as_deref() == Some("running") {
                continue;
            }

            let mut should_delete = false;

            if dry_run && older_than_days.is_none() {
                should_delete = true;
            } else if let Some(days) = older_than_days {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = std::time::SystemTime::now().duration_since(modified)
                        {
                            let threshold = std::time::Duration::from_secs(days * 24 * 3600);
                            if duration > threshold {
                                should_delete = true;
                            }
                        }
                    }
                }
            }

            if should_delete {
                if dry_run {
                    println!("dry-run: would delete {}", path.display());
                } else {
                    let _ = fs::remove_dir_all(&path);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_temp_runs_dir(prefix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "{}_{}",
            prefix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn test_format_runs_list_empty() {
        let temp_runs_dir = unique_temp_runs_dir("agentmux_runs_empty");
        let _ = fs::create_dir_all(&temp_runs_dir);

        assert_eq!(format_runs_list_from(&temp_runs_dir), "no runs found");

        let _ = fs::remove_dir_all(&temp_runs_dir);
    }

    #[test]
    fn test_runs_fixtures() {
        let runs_dir = unique_temp_runs_dir("agentmux_runs_fixture");
        let _ = fs::create_dir_all(&runs_dir);

        let unique_run_id = format!(
            "test-run-inspection-fixture-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let run_dir = runs_dir.join(&unique_run_id);
        let _ = fs::create_dir_all(&run_dir);

        let meta_content = "plan: my-plan\nagent: codex\nstatus: running\nresult: .agentmux/runs/my-run/result.md\n";
        let _ = fs::write(run_dir.join("meta.txt"), meta_content);
        let _ = fs::write(run_dir.join("transcript.ansi"), "some transcript content");
        let _ = fs::write(run_dir.join("result.md"), "# Complete Success");

        // 1. Test format_runs_list includes our fixture
        let list_str = format_runs_list_from(&runs_dir);
        assert!(list_str.contains(&unique_run_id));
        assert!(list_str.contains("running"));
        assert!(list_str.contains("codex"));
        assert!(list_str.contains("result=.agentmux/runs/my-run/result.md"));
        assert!(list_str.contains("transcript="));

        // 2. Test read_run_result gets our fixture's markdown
        let res_content = read_run_result_from(&runs_dir, &unique_run_id);
        assert_eq!(res_content, "# Complete Success");

        // 3. Clean up fixture
        let _ = fs::remove_dir_all(&runs_dir);
    }

    #[test]
    fn test_read_run_result_missing_run() {
        let runs_dir = unique_temp_runs_dir("agentmux_runs_missing");
        let _ = fs::create_dir_all(&runs_dir);

        let res = read_run_result_from(&runs_dir, "completely-nonexistent-run-id-12345");
        assert_eq!(res, "run not found: completely-nonexistent-run-id-12345");

        let _ = fs::remove_dir_all(&runs_dir);
    }

    #[test]
    fn test_read_run_result_missing_result_file() {
        let runs_dir = unique_temp_runs_dir("agentmux_runs_missing_result");
        let _ = fs::create_dir_all(&runs_dir);

        let unique_run_id = format!(
            "test-missing-res-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let run_dir = runs_dir.join(&unique_run_id);
        let _ = fs::create_dir_all(&run_dir);

        let res = read_run_result_from(&runs_dir, &unique_run_id);
        assert_eq!(res, format!("result not found: {}", unique_run_id));

        let _ = fs::remove_dir_all(&runs_dir);
    }

    #[test]
    fn test_read_run_transcript_tail_missing_run() {
        let runs_dir = unique_temp_runs_dir("agentmux_runs_tail_missing");
        let _ = fs::create_dir_all(&runs_dir);

        let res =
            read_run_transcript_tail_from(&runs_dir, "completely-nonexistent-run-id-12345", 80);
        assert_eq!(res, "run not found: completely-nonexistent-run-id-12345");

        let _ = fs::remove_dir_all(&runs_dir);
    }

    #[test]
    fn test_read_run_transcript_tail_missing_transcript() {
        let runs_dir = unique_temp_runs_dir("agentmux_runs_tail_missing_trans");
        let _ = fs::create_dir_all(&runs_dir);

        let unique_run_id = format!(
            "test-run-missing-trans-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let run_dir = runs_dir.join(&unique_run_id);
        let _ = fs::create_dir_all(&run_dir);

        let res = read_run_transcript_tail_from(&runs_dir, &unique_run_id, 80);
        assert_eq!(res, format!("transcript not found: {}", unique_run_id));

        let _ = fs::remove_dir_all(&runs_dir);
    }

    #[test]
    fn test_read_run_transcript_tail_success() {
        let runs_dir = unique_temp_runs_dir("agentmux_runs_tail_success");
        let _ = fs::create_dir_all(&runs_dir);

        let unique_run_id = format!(
            "test-run-tail-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let run_dir = runs_dir.join(&unique_run_id);
        let _ = fs::create_dir_all(&run_dir);

        // Create a transcript file with 10 lines
        let mut transcript_lines = Vec::new();
        for i in 1..=10 {
            transcript_lines.push(format!("line {}", i));
        }
        let transcript_content = transcript_lines.join("\n");
        let _ = fs::write(run_dir.join("transcript.ansi"), transcript_content);

        // Test taking last 5 lines
        let res = read_run_transcript_tail_from(&runs_dir, &unique_run_id, 5);
        let expected_header = format!("transcript tail: {} last 5 lines", unique_run_id);
        let expected_body = "line 6\nline 7\nline 8\nline 9\nline 10";
        assert_eq!(res, format!("{}\n{}", expected_header, expected_body));

        // Test taking 20 lines (more than exists)
        let res_more = read_run_transcript_tail_from(&runs_dir, &unique_run_id, 20);
        let expected_header_more = format!("transcript tail: {} last 20 lines", unique_run_id);
        let expected_body_more =
            "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10";
        assert_eq!(
            res_more,
            format!("{}\n{}", expected_header_more, expected_body_more)
        );

        let _ = fs::remove_dir_all(&runs_dir);
    }

    #[test]
    fn test_read_run_transcript_tail_cap() {
        let runs_dir = unique_temp_runs_dir("agentmux_runs_tail_cap");
        let _ = fs::create_dir_all(&runs_dir);

        let unique_run_id = format!(
            "test-run-tail-cap-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let run_dir = runs_dir.join(&unique_run_id);
        let _ = fs::create_dir_all(&run_dir);

        // Create a transcript file with 600 lines
        let mut transcript_lines = Vec::new();
        for i in 1..=600 {
            transcript_lines.push(format!("line {}", i));
        }
        let transcript_content = transcript_lines.join("\n");
        let _ = fs::write(run_dir.join("transcript.ansi"), transcript_content);

        // Request 1000 lines, should cap to 500
        let res = read_run_transcript_tail_from(&runs_dir, &unique_run_id, 1000);
        let expected_header = format!("transcript tail: {} last 500 lines", unique_run_id);
        // last 500 lines start from line 101 to 600
        let mut expected_lines = Vec::new();
        for i in 101..=600 {
            expected_lines.push(format!("line {}", i));
        }
        assert_eq!(
            res,
            format!("{}\n{}", expected_header, expected_lines.join("\n"))
        );

        let _ = fs::remove_dir_all(&runs_dir);
    }

    #[test]
    fn test_cleanup_runs_invalid_args() {
        let runs_dir = unique_temp_runs_dir("agentmux_runs_cleanup_invalid");
        let _ = fs::create_dir_all(&runs_dir);

        // Missing args
        let res1 = cleanup_runs_from(&runs_dir, &[]);
        assert!(res1.is_err());
        assert_eq!(
            res1.unwrap_err(),
            "usage: agentmux cleanup-runs --dry-run|--older-than <days>"
        );

        // Mismatched option
        let res2 = cleanup_runs_from(&runs_dir, &["--invalid".to_string()]);
        assert!(res2.is_err());

        // Older than 0
        let res3 = cleanup_runs_from(&runs_dir, &["--older-than".to_string(), "0".to_string()]);
        assert!(res3.is_err());

        let _ = fs::remove_dir_all(&runs_dir);
    }

    #[test]
    fn test_cleanup_runs_dry_run() {
        let runs_dir = unique_temp_runs_dir("agentmux_runs_cleanup_dry");
        let _ = fs::create_dir_all(&runs_dir);

        let run_id = "test-run-cleanup-dry";
        let run_dir = runs_dir.join(run_id);
        let _ = fs::create_dir_all(&run_dir);

        // Run dry-run, folder must not be deleted
        let res = cleanup_runs_from(&runs_dir, &["--dry-run".to_string()]);
        assert!(res.is_ok());
        assert!(run_dir.exists());

        let _ = fs::remove_dir_all(&runs_dir);
    }

    #[test]
    fn test_cleanup_runs_skips_running_meta() {
        let runs_dir = unique_temp_runs_dir("agentmux_runs_cleanup_running");
        let _ = fs::create_dir_all(&runs_dir);

        let running_dir = runs_dir.join("test-run-running-meta");
        let stopped_dir = runs_dir.join("test-run-stopped-meta");
        let _ = fs::create_dir_all(&running_dir);
        let _ = fs::create_dir_all(&stopped_dir);
        let _ = fs::write(running_dir.join("meta.txt"), "status: running\n");
        let _ = fs::write(stopped_dir.join("meta.txt"), "status: stopped\n");

        let res = cleanup_runs_from(&runs_dir, &["--older-than".to_string(), "1".to_string()]);
        assert!(res.is_ok());
        assert!(running_dir.exists());

        let _ = fs::remove_dir_all(&runs_dir);
    }
}
