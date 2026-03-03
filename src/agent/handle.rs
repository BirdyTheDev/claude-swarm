use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::{error, info};

use super::subprocess;
use crate::types::agent::{AgentConfig, AgentId};
use crate::types::message::ClaudeStreamMessage;

/// High-level interface to a Claude agent.
/// Each prompt spawns a new `claude -p` process, using `--resume` to continue the session.
pub struct AgentHandle {
    pub id: AgentId,
    pub config: AgentConfig,
    session_id: Arc<Mutex<Option<String>>>,
    output_tx: mpsc::Sender<(AgentId, ClaudeStreamMessage)>,
    busy: Arc<Mutex<bool>>,
    /// If set, opens a visible Terminal.app window instead of headless subprocess.
    pub visible_terminal: bool,
    /// Directory for log files when using visible terminals.
    pub log_dir: Option<PathBuf>,
}

impl AgentHandle {
    pub fn new(
        id: AgentId,
        config: AgentConfig,
        output_tx: mpsc::Sender<(AgentId, ClaudeStreamMessage)>,
    ) -> Self {
        Self {
            id,
            config,
            session_id: Arc::new(Mutex::new(None)),
            output_tx,
            busy: Arc::new(Mutex::new(false)),
            visible_terminal: false,
            log_dir: None,
        }
    }

    pub fn with_visible_terminal(mut self, log_dir: PathBuf) -> Self {
        self.visible_terminal = true;
        self.log_dir = Some(log_dir);
        self
    }

    /// Send a prompt to this agent. Spawns a claude process, waits for completion.
    /// This runs in a spawned task so it doesn't block the caller.
    pub fn send_prompt(&self, prompt: String) {
        let id = self.id.clone();
        let config = self.config.clone();
        let session_id = self.session_id.clone();
        let output_tx = self.output_tx.clone();
        let busy = self.busy.clone();
        let visible_terminal = self.visible_terminal;
        let log_dir = self.log_dir.clone();

        tokio::spawn(async move {
            // Mark as busy
            {
                let mut b = busy.lock().await;
                if *b {
                    info!(agent = %id, "agent busy, queuing skipped");
                    return;
                }
                *b = true;
            }

            let sid = session_id.lock().await.clone();

            let result = if visible_terminal {
                if let Some(ref dir) = log_dir {
                    subprocess::run_prompt_in_terminal(
                        &id,
                        &config,
                        &prompt,
                        sid.as_deref(),
                        dir,
                        output_tx,
                    )
                    .await
                } else {
                    subprocess::run_prompt(&id, &config, &prompt, sid.as_deref(), output_tx).await
                }
            } else {
                subprocess::run_prompt(&id, &config, &prompt, sid.as_deref(), output_tx).await
            };

            match result {
                Ok(new_sid) => {
                    // Store session_id for resumption
                    if let Some(new_sid) = new_sid {
                        *session_id.lock().await = Some(new_sid);
                    }
                }
                Err(e) => {
                    error!(agent = %id, "prompt failed: {e}");
                }
            }

            *busy.lock().await = false;
        });
    }

    pub async fn is_busy(&self) -> bool {
        *self.busy.lock().await
    }

    pub async fn session_id(&self) -> Option<String> {
        self.session_id.lock().await.clone()
    }
}
