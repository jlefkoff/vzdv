//! Session-backed flashed messages to the user.

use crate::shared::SESSION_FLASHED_MESSAGES_KEY;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

/// Stored in the session. Contains pending flashed messages, if any.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FlashedMessages(Vec<FlashedMessage>);

/// Message significance. Dictates the CSS classes used to render the alert.
#[derive(Debug, Serialize, Deserialize)]
pub enum FlashedMessageLevel {
    Info,
    Success,
    Error,
}

/// A single message to show to the user.
#[derive(Debug, Serialize, Deserialize)]
pub struct FlashedMessage {
    pub level: FlashedMessageLevel,
    pub message: String,
    pub class: String,
}

impl FlashedMessage {
    /// Create a new message to be shown to the user.
    pub fn new(level: FlashedMessageLevel, message: &str) -> Self {
        let class_suffix = match level {
            FlashedMessageLevel::Info => "info",
            FlashedMessageLevel::Success => "success",
            FlashedMessageLevel::Error => "error",
        };
        let class = format!("alert alert-{class_suffix}");
        Self {
            level,
            message: message.to_owned(),
            class,
        }
    }

    /// Get the CSS classes for the level for use in templates.
    #[allow(unused)]
    pub fn class(self) -> String {
        let s = match self.level {
            FlashedMessageLevel::Info => "info",
            FlashedMessageLevel::Success => "success",
            FlashedMessageLevel::Error => "error",
        };
        format!("alert alert-{s}")
    }
}

// TODO do I care that flashed messages go to the DB, or should
// I try to have 2 sessions: 1 backed by the DB, and the other
// just in memory?

/// Push a session message to be flashed to the user.
pub async fn push_flashed_message(
    session: Session,
    level: FlashedMessageLevel,
    message: &str,
) -> Result<()> {
    let new_message = FlashedMessage::new(level, message);
    let messages = match session
        .get::<FlashedMessages>(SESSION_FLASHED_MESSAGES_KEY)
        .await?
    {
        Some(mut messages) => {
            messages.0.push(new_message);
            messages
        }
        None => FlashedMessages(vec![new_message]),
    };
    session
        .insert(SESSION_FLASHED_MESSAGES_KEY, messages)
        .await?;
    Ok(())
}

/// Collect the flashed messages from the user's session and return them.
///
/// The returned messages are removed from the users's session.
pub async fn drain_flashed_messages(session: Session) -> Result<Vec<FlashedMessage>> {
    if let Some(messages) = session
        .get::<FlashedMessages>(SESSION_FLASHED_MESSAGES_KEY)
        .await?
    {
        let ret = messages.0;
        session.remove_value(SESSION_FLASHED_MESSAGES_KEY).await?;
        Ok(ret)
    } else {
        Ok(Vec::new())
    }
}
