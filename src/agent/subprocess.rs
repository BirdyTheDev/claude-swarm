use anyhow::{Context, Result};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::types::agent::{AgentConfig, AgentId};
use crate::types::message::ClaudeStreamMessage;

/// Runs a one-shot `claude -p "prompt" --output-format stream-json --verbose` invocation.
/// Returns the session_id from the system init message (for --resume on next call).
pub async fn run_prompt(
    id: &AgentId,
    config: &AgentConfig,
    prompt: &str,
    session_id: Option<&str>,
    output_tx: mpsc::Sender<(AgentId, ClaudeStreamMessage)>,
) -> Result<Option<String>> {
    let args = config.to_cli_args(prompt, session_id);
    debug!(agent = %id, args = ?args, "running claude prompt");

    let mut child = Command::new("claude")
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env_remove("CLAUDECODE")
        .spawn()
        .with_context(|| format!("spawning claude for agent '{id}'"))?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    // Track session_id from system init
    let (sid_tx, mut sid_rx) = mpsc::channel::<String>(1);

    // Stdout reader task - parses NDJSON
    let agent_id_out = id.clone();
    let out_tx = output_tx.clone();
    let reader_handle = tokio::spawn(async move {
        let reader = tokio::io::BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            match ClaudeStreamMessage::parse(&line) {
                Some(msg) => {
                    // Extract session_id from system init
                    if let ClaudeStreamMessage::System(ref sys) = msg {
                        if let Some(ref sid) = sys.session_id {
                            let _ = sid_tx.try_send(sid.clone());
                        }
                    }
                    if out_tx.send((agent_id_out.clone(), msg)).await.is_err() {
                        break;
                    }
                }
                None => {
                    warn!(agent = %agent_id_out, "unparseable stream line: {line}");
                }
            }
        }
    });

    // Stderr reader task
    let agent_id_err = id.clone();
    tokio::spawn(async move {
        let reader = tokio::io::BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim().to_string();
            if !line.is_empty() {
                warn!(agent = %agent_id_err, "stderr: {line}");
            }
        }
    });

    // Wait for the process to complete
    let status = child.wait().await?;
    // Wait for reader to finish processing
    let _ = reader_handle.await;

    if !status.success() {
        info!(agent = %id, code = ?status.code(), "claude process exited with error");
    }

    // Get session_id if we received one
    let new_session_id = sid_rx.try_recv().ok();
    Ok(new_session_id)
}

/// Runs a claude prompt in a visible Terminal.app window using osascript (macOS).
/// The output is tee'd to a log file which is then tailed for NDJSON parsing.
/// Returns the session_id from the system init message.
pub async fn run_prompt_in_terminal(
    id: &AgentId,
    config: &AgentConfig,
    prompt: &str,
    session_id: Option<&str>,
    log_dir: &std::path::Path,
    output_tx: mpsc::Sender<(AgentId, ClaudeStreamMessage)>,
) -> Result<Option<String>> {
    let args = config.to_cli_args(prompt, session_id);
    debug!(agent = %id, args = ?args, "running claude in visible terminal");

    // Create log file path
    let log_file = log_dir.join(format!("{}.ndjson", id.0));

    // Build the shell command: claude <args> | tee <log_file>
    let escaped_args: Vec<String> = args
        .iter()
        .map(|a| shell_escape::escape(std::borrow::Cow::Borrowed(a)).to_string())
        .collect();
    let shell_cmd = format!(
        "claude {} 2>/dev/null | tee {}",
        escaped_args.join(" "),
        shell_escape::escape(std::borrow::Cow::Borrowed(&log_file.to_string_lossy())),
    );

    // Use osascript to open a new Terminal.app window
    let title = format!("claude-swarm: {}", id.0);
    let osascript = format!(
        r#"tell application "Terminal"
    activate
    set newTab to do script "{cmd}"
    set custom title of newTab to "{title}"
end tell"#,
        cmd = shell_cmd.replace('\\', "\\\\").replace('"', "\\\""),
        title = title.replace('"', "\\\""),
    );

    // Ensure log file exists (create empty)
    if let Some(parent) = log_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&log_file, "")?;

    // Launch osascript
    let status = Command::new("osascript")
        .arg("-e")
        .arg(&osascript)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .with_context(|| format!("launching Terminal.app for agent '{id}'"))?;

    if !status.success() {
        warn!(agent = %id, "osascript failed to open terminal window");
    }

    // Now tail the log file for NDJSON output
    let agent_id_tail = id.clone();
    let (sid_tx, mut sid_rx) = mpsc::channel::<String>(1);
    let log_file_clone = log_file.clone();

    let reader_handle = tokio::spawn(async move {
        // Wait briefly for the file to start getting written
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Open file and seek to beginning, then continuously read new lines
        let file = match tokio::fs::File::open(&log_file_clone).await {
            Ok(f) => f,
            Err(e) => {
                warn!(agent = %agent_id_tail, "failed to open log file: {e}");
                return;
            }
        };

        let reader = tokio::io::BufReader::new(file);
        let mut lines = reader.lines();
        let mut empty_count = 0u32;

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    empty_count = 0;
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    match ClaudeStreamMessage::parse(&line) {
                        Some(msg) => {
                            if let ClaudeStreamMessage::System(ref sys) = msg {
                                if let Some(ref sid) = sys.session_id {
                                    let _ = sid_tx.try_send(sid.clone());
                                }
                            }
                            // Check if this is a result message (signals completion)
                            let is_result = matches!(msg, ClaudeStreamMessage::Result(_));
                            if output_tx
                                .send((agent_id_tail.clone(), msg))
                                .await
                                .is_err()
                            {
                                break;
                            }
                            if is_result {
                                break;
                            }
                        }
                        None => {
                            warn!(agent = %agent_id_tail, "unparseable stream line from log: {line}");
                        }
                    }
                }
                Ok(None) => {
                    // No more lines available, wait and try again
                    empty_count += 1;
                    if empty_count > 600 {
                        // 5 minutes with no output, give up
                        info!(agent = %agent_id_tail, "log tail timed out");
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                Err(e) => {
                    warn!(agent = %agent_id_tail, "log read error: {e}");
                    break;
                }
            }
        }
    });

    // Wait for the reader to finish (this blocks until result message or timeout)
    let _ = reader_handle.await;

    let new_session_id = sid_rx.try_recv().ok();
    Ok(new_session_id)
}
