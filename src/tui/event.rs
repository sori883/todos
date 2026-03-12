use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};

use crate::error::AppError;

/// Application events.
pub enum AppEvent {
    /// A key was pressed.
    Key(KeyEvent),
    /// A tick interval elapsed.
    Tick,
    /// The terminal was resized.
    Resize(u16, u16),
}

/// Handles crossterm events in a background thread and sends them as AppEvents.
pub struct EventHandler {
    rx: mpsc::Receiver<AppEvent>,
    _tx: mpsc::Sender<AppEvent>,
}

impl EventHandler {
    /// Create a new EventHandler with the given tick rate in milliseconds.
    pub fn new(tick_rate_ms: u64) -> Self {
        let (tx, rx) = mpsc::channel();
        let event_tx = tx.clone();
        let tick_rate = Duration::from_millis(tick_rate_ms);

        thread::spawn(move || {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    match event::read() {
                        Ok(Event::Key(key)) => {
                            if event_tx.send(AppEvent::Key(key)).is_err() {
                                return;
                            }
                        }
                        Ok(Event::Resize(w, h)) => {
                            if event_tx.send(AppEvent::Resize(w, h)).is_err() {
                                return;
                            }
                        }
                        _ => {}
                    }
                } else {
                    // No event within tick_rate, send a Tick
                    if event_tx.send(AppEvent::Tick).is_err() {
                        return;
                    }
                }
            }
        });

        Self { rx, _tx: tx }
    }

    /// Get the next event, blocking until one is available.
    pub fn next(&self) -> Result<AppEvent, AppError> {
        self.rx
            .recv()
            .map_err(|e| AppError::DataFile(format!("Event channel error: {e}")))
    }
}
