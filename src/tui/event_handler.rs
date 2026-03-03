use anyhow::Result;
use crossterm::event::{Event, EventStream};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::types::event::AppEvent;

/// Reads crossterm events and converts them to AppEvents.
pub struct EventHandler {
    tx: mpsc::Sender<AppEvent>,
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tx: mpsc::Sender<AppEvent>, tick_rate_ms: u64) -> Self {
        Self {
            tx,
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    /// Spawn a background task that reads terminal events.
    pub fn spawn(self) -> tokio::task::JoinHandle<Result<()>> {
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = tokio::time::interval(self.tick_rate);

            loop {
                tokio::select! {
                    _ = tick_interval.tick() => {
                        if self.tx.send(AppEvent::Tick).await.is_err() {
                            break;
                        }
                    }
                    event = reader.next() => {
                        match event {
                            Some(Ok(Event::Key(key))) => {
                                if self.tx.send(AppEvent::Key(key)).await.is_err() {
                                    break;
                                }
                            }
                            Some(Ok(Event::Resize(w, h))) => {
                                if self.tx.send(AppEvent::Resize(w, h)).await.is_err() {
                                    break;
                                }
                            }
                            Some(Err(_)) | None => break,
                            _ => {}
                        }
                    }
                }
            }

            Ok(())
        })
    }
}
