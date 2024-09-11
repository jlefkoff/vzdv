//! Session-backed flashed messages to the user.

use crate::shared::{AppError, SESSION_FLASHED_MESSAGES_KEY};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

/**
 * TODO
 *
 * Flashed messages don't show on redirects until the user navigates to another
 * page and renders that. Is there some weird lagging session update somewhere?
 * I tried explicitly adding a `.save` call, but that doesn't seem to have fixed it.
 */

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

impl FlashedMessageLevel {
    /// String representation, suitable for use in templates.
    pub fn as_str(&self) -> &'static str {
        match self {
            FlashedMessageLevel::Info => "info",
            FlashedMessageLevel::Success => "success",
            FlashedMessageLevel::Error => "danger",
        }
    }
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
        let class = format!("alert alert-{}", level.as_str());
        Self {
            level,
            message: message.to_owned(),
            class,
        }
    }

    /// Get the CSS classes for the level for use in templates.
    #[allow(unused)]
    pub fn class(self) -> String {
        format!("alert alert-{}", self.level.as_str())
    }
}

/// Push a session message to be flashed to the user.
pub async fn push_flashed_message(
    session: Session,
    level: FlashedMessageLevel,
    message: &str,
) -> Result<(), AppError> {
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
    session.save().await.unwrap();
    Ok(())
}

/// Collect the flashed messages from the user's session and return them.
///
/// The returned messages are removed from the users's session.
pub async fn drain_flashed_messages(session: Session) -> Result<Vec<FlashedMessage>, AppError> {
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
