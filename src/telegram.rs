use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::types::agent::AgentId;
use crate::types::event::{AppEvent, OrchestratorCommand};

/// Notification messages sent to Telegram.
#[derive(Debug, Clone)]
pub enum TelegramNotification {
    Text(String),
}

/// Bridges between Telegram bot API and the swarm orchestrator.
pub struct TelegramBridge {
    bot_token: String,
    chat_id: Option<i64>,
    pairing_code: Option<String>,
    paired: Arc<Mutex<Option<i64>>>,
    cmd_tx: mpsc::Sender<OrchestratorCommand>,
    event_tx: mpsc::Sender<AppEvent>,
    notify_rx: mpsc::Receiver<TelegramNotification>,
}

impl TelegramBridge {
    pub fn new(
        bot_token: String,
        chat_id: Option<i64>,
        cmd_tx: mpsc::Sender<OrchestratorCommand>,
        event_tx: mpsc::Sender<AppEvent>,
        notify_rx: mpsc::Receiver<TelegramNotification>,
    ) -> Self {
        let (pairing_code, paired) = if let Some(id) = chat_id {
            (None, Arc::new(Mutex::new(Some(id))))
        } else {
            use rand::Rng;
            let code: u32 = rand::thread_rng().gen_range(100_000..1_000_000);
            (Some(code.to_string()), Arc::new(Mutex::new(None)))
        };
        Self {
            bot_token,
            chat_id,
            pairing_code,
            paired,
            cmd_tx,
            event_tx,
            notify_rx,
        }
    }

    /// Returns the pairing code if in pairing mode (chat_id was empty).
    pub fn pairing_code(&self) -> Option<&str> {
        self.pairing_code.as_deref()
    }

    /// Run the Telegram bridge: polling for incoming messages + sending notifications.
    pub async fn run(self) {
        use teloxide::prelude::*;
        use teloxide::types::ChatId;

        let bot = Bot::new(&self.bot_token);
        let paired = self.paired.clone();

        // If already paired, log it
        if self.chat_id.is_some() {
            let id = self.chat_id.unwrap();
            info!("telegram bridge starting for chat_id {id}");
            let _ = self
                .event_tx
                .send(AppEvent::TelegramNotify {
                    text: "Telegram bridge connected".to_string(),
                })
                .await;
        } else {
            let code = self.pairing_code.as_deref().unwrap_or("???");
            info!("telegram bridge starting in pairing mode, code: {code}");
            let _ = self
                .event_tx
                .send(AppEvent::TelegramNotify {
                    text: format!("Pairing code: {code} — send to your Telegram bot"),
                })
                .await;
        }

        // Spawn notification sender — only sends to paired chat
        let notify_bot = bot.clone();
        let notify_paired = paired.clone();
        let mut notify_rx = self.notify_rx;
        tokio::spawn(async move {
            while let Some(notification) = notify_rx.recv().await {
                let TelegramNotification::Text(text) = notification;
                let target = { *notify_paired.lock().unwrap() };
                if let Some(chat_id) = target {
                    if let Err(e) = notify_bot.send_message(ChatId(chat_id), &text).await {
                        warn!("telegram send failed: {e}");
                    }
                }
            }
        });

        // Polling loop for incoming messages
        let cmd_tx = self.cmd_tx;
        let event_tx = self.event_tx;
        let pairing_code = self.pairing_code.clone();

        teloxide::repl(bot, move |message: Message, bot: Bot| {
            let cmd_tx = cmd_tx.clone();
            let event_tx = event_tx.clone();
            let paired = paired.clone();
            let pairing_code = pairing_code.clone();
            async move {
                let text = match message.text() {
                    Some(t) => t.trim().to_string(),
                    None => return Ok(()),
                };
                let sender_chat_id = message.chat.id.0;

                // Check pairing state
                let current_paired = { *paired.lock().unwrap() };

                match current_paired {
                    None => {
                        // Not yet paired — check if message matches pairing code
                        if let Some(ref code) = pairing_code {
                            if text == *code {
                                // Pair!
                                {
                                    *paired.lock().unwrap() = Some(sender_chat_id);
                                }
                                let _ = bot
                                    .send_message(
                                        message.chat.id,
                                        "Paired! Now accepting commands.",
                                    )
                                    .await;
                                let _ = event_tx
                                    .send(AppEvent::TelegramPaired {
                                        chat_id: sender_chat_id.to_string(),
                                    })
                                    .await;
                                info!("telegram paired with chat_id {sender_chat_id}");
                            } else {
                                let _ = bot
                                    .send_message(
                                        message.chat.id,
                                        "Send the pairing code shown in TUI to connect.",
                                    )
                                    .await;
                            }
                        }
                    }
                    Some(expected_id) => {
                        // Already paired — only accept from paired chat
                        if sender_chat_id != expected_id {
                            return Ok(());
                        }

                        let command = parse_telegram_command(&text);
                        match command {
                            TelegramCommand::SendPrompt { agent_id, prompt } => {
                                // Route through event_tx so app.rs creates a Task entry
                                let _ = event_tx
                                    .send(AppEvent::TelegramTaskPrompt {
                                        agent_id: AgentId::new(&agent_id),
                                        prompt,
                                    })
                                    .await;
                            }
                            TelegramCommand::TeamTask { description } => {
                                // Route through event_tx so app.rs creates a Task entry
                                let _ = event_tx
                                    .send(AppEvent::TelegramTeamTask { description })
                                    .await;
                            }
                            TelegramCommand::Broadcast { prompt } => {
                                let _ = cmd_tx
                                    .send(OrchestratorCommand::Broadcast { prompt })
                                    .await;
                            }
                            TelegramCommand::StopAgent { agent_id } => {
                                let _ = cmd_tx
                                    .send(OrchestratorCommand::StopAgent {
                                        agent_id: AgentId::new(&agent_id),
                                    })
                                    .await;
                            }
                            TelegramCommand::Shutdown => {
                                let _ = cmd_tx.send(OrchestratorCommand::Shutdown).await;
                            }
                            TelegramCommand::PromptLead { prompt } => {
                                let _ = cmd_tx
                                    .send(OrchestratorCommand::PromptLead { prompt })
                                    .await;
                            }
                            TelegramCommand::Status => {
                                let _ = event_tx
                                    .send(AppEvent::TelegramStatusRequest)
                                    .await;
                            }
                            TelegramCommand::Cost => {
                                let _ = event_tx
                                    .send(AppEvent::TelegramCostRequest)
                                    .await;
                            }
                            TelegramCommand::SetSoul { agent_id, soul } => {
                                let _ = cmd_tx
                                    .send(OrchestratorCommand::SetSoul {
                                        agent_id: AgentId::new(&agent_id),
                                        soul,
                                    })
                                    .await;
                            }
                            TelegramCommand::Schedule { time, command } => {
                                let _ = event_tx
                                    .send(AppEvent::TelegramSchedule { time, command })
                                    .await;
                            }
                            TelegramCommand::SchedulesList => {
                                let _ = event_tx
                                    .send(AppEvent::TelegramSchedulesList)
                                    .await;
                            }
                        }

                        let _ = event_tx
                            .send(AppEvent::TelegramNotify {
                                text: format!("TG: {text}"),
                            })
                            .await;
                    }
                }

                Ok(())
            }
        })
        .await;
    }
}

enum TelegramCommand {
    SendPrompt { agent_id: String, prompt: String },
    TeamTask { description: String },
    Broadcast { prompt: String },
    StopAgent { agent_id: String },
    Shutdown,
    PromptLead { prompt: String },
    Status,
    Cost,
    SetSoul { agent_id: String, soul: String },
    Schedule { time: String, command: String },
    SchedulesList,
}

fn parse_telegram_command(text: &str) -> TelegramCommand {
    let trimmed = text.trim();

    if let Some(rest) = trimmed.strip_prefix(":t ") {
        // :t <agent> <prompt>
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        if parts.len() == 2 {
            return TelegramCommand::SendPrompt {
                agent_id: parts[0].to_string(),
                prompt: parts[1].to_string(),
            };
        }
    }

    if let Some(rest) = trimmed.strip_prefix(":tt ") {
        return TelegramCommand::TeamTask {
            description: rest.to_string(),
        };
    }

    if let Some(rest) = trimmed.strip_prefix(":bc ") {
        return TelegramCommand::Broadcast {
            prompt: rest.to_string(),
        };
    }

    if let Some(rest) = trimmed.strip_prefix(":stop ") {
        return TelegramCommand::StopAgent {
            agent_id: rest.trim().to_string(),
        };
    }

    if let Some(rest) = trimmed.strip_prefix(":soul ") {
        // :soul <agent> <text>
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        if parts.len() == 2 {
            return TelegramCommand::SetSoul {
                agent_id: parts[0].to_string(),
                soul: parts[1].to_string(),
            };
        }
    }

    if let Some(rest) = trimmed.strip_prefix(":schedule ") {
        // :schedule HH:MM <command>
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        if parts.len() == 2 {
            return TelegramCommand::Schedule {
                time: parts[0].to_string(),
                command: parts[1].to_string(),
            };
        }
    }

    if trimmed == ":schedules" {
        return TelegramCommand::SchedulesList;
    }

    if trimmed == ":status" {
        return TelegramCommand::Status;
    }

    if trimmed == ":cost" {
        return TelegramCommand::Cost;
    }

    if trimmed == ":q" {
        return TelegramCommand::Shutdown;
    }

    // Default: send to lead agent
    TelegramCommand::PromptLead {
        prompt: trimmed.to_string(),
    }
}
