//! The chat plugin: messages that the members of an install send to each other.
//!
//! PHP Organizr stores the messages in the table `chatroom` and pushes each new one to
//! the browsers through Pusher, a service of another company. Here the server pushes
//! them itself, with server-sent events at `GET /plugins/chat/events` (D-44). The
//! messages are plain text (D-24). `docs/deviations.md` lists the differences as D-95.

use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::Stream;
use organizr_domain::{ChatMessage, GroupLevel};
use organizr_settings::Settings;
use serde::Serialize;
use serde_json::{Map, Value, json};
use tokio::sync::{broadcast, watch};

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::{Envelope, NoData};
use crate::handlers::fields::{Refusal, is_nothing_in_php, php_text};
use crate::handlers::invites::date_and_time;
use crate::state::AppState;

/// The level of the settings route: Admins and Co-Admins, as `qualifyRequest(1)`.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// The most characters of a message.
pub const MESSAGE_LIMIT: usize = 1000;

/// The most messages that `GET /plugins/chat/message` sends, whatever the setting
/// `CHAT-messageLoadLimit` says.
const MOST_LOADED: u64 = 1000;

/// The most messages that one sender may send within [`SEND_WINDOW`].
const MOST_SENT: usize = 10;

/// The time over which [`MOST_SENT`] counts.
const SEND_WINDOW: Duration = Duration::from_secs(30);

/// How long one stream of events lasts. The browser then opens a new one, and the level
/// of its session is checked again.
const STREAM_LIFETIME: Duration = Duration::from_secs(300);

/// How many new messages wait for a slow stream before it skips them.
const WAITING_MESSAGES: usize = 64;

/// What the chat routes share: the channel of new messages, and the times of the last
/// messages of each sender.
#[derive(Debug)]
pub struct ChatHub {
    messages: broadcast::Sender<Arc<str>>,
    closing: watch::Sender<bool>,
    sent: std::sync::Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl Default for ChatHub {
    fn default() -> Self {
        Self {
            messages: broadcast::channel(WAITING_MESSAGES).0,
            closing: watch::channel(false).0,
            sent: std::sync::Mutex::default(),
        }
    }
}

impl ChatHub {
    /// Ends every stream of events, for the end of the server: an open stream would
    /// keep the server from stopping.
    pub fn close(&self) {
        self.closing.send_replace(true);
    }

    /// Counts a message of a sender, unless the sender sent too many within the window.
    /// Returns whether the message may go.
    fn may_send(&self, sender: &str, now: Instant) -> bool {
        let Ok(mut sent) = self.sent.lock() else {
            return false;
        };
        // Times that fell out of the window are forgotten, and so is a sender without
        // a time in it.
        sent.retain(|_, times| {
            while times
                .front()
                .is_some_and(|time| now.duration_since(*time) >= SEND_WINDOW)
            {
                times.pop_front();
            }
            !times.is_empty()
        });
        let times = sent.entry(sender.to_owned()).or_default();
        if times.len() >= MOST_SENT {
            return false;
        }
        times.push_back(now);
        true
    }
}

/// A message as the API shows it, with the fields of PHP. The address of the sender is
/// never shown.
#[derive(Debug, Serialize)]
pub struct MessageRow {
    username: String,
    gravatar: String,
    uid: String,
    date: String,
    message: String,
}

impl From<ChatMessage> for MessageRow {
    fn from(message: ChatMessage) -> Self {
        Self {
            username: message.username,
            gravatar: message.gravatar,
            uid: message.uid,
            date: date_and_time(message.sent_at),
            message: message.message,
        }
    }
}

/// The plugin answers only when it is switched on.
fn check_enabled(settings: &Settings) -> Result<(), Refusal> {
    if settings.chat_enabled {
        Ok(())
    } else {
        Err(Refusal::conflict("CHAT-enabled module is not enabled"))
    }
}

/// The level that reads and sends messages: the setting `CHAT-Auth-include`. A guest
/// never does, whatever the setting says: the level of guests counts as "logged in".
fn reader_level(settings: &Settings) -> GroupLevel {
    let level = u16::try_from(settings.chat_auth_include).unwrap_or(GroupLevel::ADMIN.get());
    GroupLevel::new(level.min(GroupLevel::LOGGED_IN.get())).unwrap_or(GroupLevel::ADMIN)
}

/// The checks of the routes that read and send messages.
fn check_reader(settings: &Settings, caller: &Caller) -> Result<(), Refusal> {
    check_enabled(settings)?;
    caller.require(reader_level(settings))?;
    Ok(())
}

/// `GET /plugins/chat/message`: the newest messages, the newest first, at most
/// `CHAT-messageLoadLimit` of them.
pub async fn messages(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Vec<MessageRow>>, Refusal> {
    let settings = state.settings();
    check_reader(&settings, &caller)?;
    let limit = u64::from(settings.chat_message_load_limit).min(MOST_LOADED);
    let messages = state.database().chat().newest(limit).await?;
    Ok(Envelope::success(
        messages.into_iter().map(MessageRow::from).collect(),
    ))
}

/// `POST /plugins/chat/message`: sends a message. The field `message` holds the text.
pub async fn send(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    let settings = state.settings();
    check_reader(&settings, &caller)?;
    // What PHP takes for "no message": no value, the empty text, the text `0`.
    let message = data
        .field("message")
        .filter(|value| !is_nothing_in_php(value))
        .and_then(php_text)
        .ok_or_else(|| Refusal::conflict("No message supplied"))?;
    if message.chars().count() > MESSAGE_LIMIT {
        return Err(Refusal::over_limit(MESSAGE_LIMIT));
    }

    // A caller with the API key and no session writes as PHP names it.
    let (sender, username, gravatar, uid) = caller.user().map_or_else(
        || {
            (
                "\0api key".to_owned(),
                "Organizr API".to_owned(),
                String::new(),
                String::new(),
            )
        },
        |user| {
            let user = &user.authenticated.user;
            (
                format!("user {}", user.id.get()),
                user.username.clone(),
                user.image.clone().unwrap_or_default(),
                user.id.get().to_string(),
            )
        },
    );
    let hub = state.chat_hub();
    if !hub.may_send(&sender, Instant::now()) {
        return Err(Refusal::Request(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many chat messages. Wait a moment, then send again.".to_owned(),
        ));
    }

    let sent_at = jiff::Timestamp::now().as_second();
    let stored = state
        .database()
        .chat()
        .insert(ChatMessage {
            id: 0,
            username,
            gravatar,
            uid,
            sent_at,
            ip: client.address.to_string(),
            message,
        })
        .await?;
    match serde_json::to_string(&MessageRow::from(stored)) {
        // Nobody may be listening, which is no failure.
        Ok(event) => drop(hub.messages.send(Arc::from(event))),
        Err(error) => tracing::error!(%error, "a chat message could not be written as JSON"),
    }
    Ok(Envelope::success(NoData).with_message("Chat message accepted"))
}

/// `GET /plugins/chat/events`: the new messages as server-sent events, each one an event
/// `message` whose data is the message as `GET /plugins/chat/message` shows it. Not a
/// route of PHP, which pushes the messages through Pusher (D-44).
pub async fn events(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Refusal> {
    check_reader(&state.settings(), &caller)?;
    let hub = state.chat_hub();
    let receiver = hub.messages.subscribe();
    let closing = hub.closing.subscribe();
    let deadline = tokio::time::Instant::now() + STREAM_LIFETIME;
    let stream = futures_util::stream::unfold(
        (receiver, closing),
        move |(mut receiver, mut closing)| async move {
            loop {
                if *closing.borrow() {
                    return None;
                }
                tokio::select! {
                    () = tokio::time::sleep_until(deadline) => return None,
                    changed = closing.changed() => {
                        if changed.is_err() {
                            return None;
                        }
                    }
                    message = receiver.recv() => match message {
                        Ok(message) => {
                            let event = Event::default().event("message").data(&*message);
                            return Some((Ok(event), (receiver, closing)));
                        }
                        // A stream that fell behind skips what it missed.
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => return None,
                    },
                }
            }
        },
    );
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// `GET /plugins/chat/settings`: the form of the settings of the plugin, as PHP's
/// `_chatPluginGetSettings()` gives it, without its HTML (D-22) and without the fields
/// that have no effect here: the sound, the time between two loads and the settings of
/// Pusher, which this application does not use (D-44, D-95).
pub async fn settings(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Map<String, Value>>, Refusal> {
    caller.require(MANAGER)?;
    let settings = state.settings();
    let groups: Vec<Value> = state
        .database()
        .groups()
        .list()
        .await?
        .into_iter()
        .map(|group| json!({ "name": group.name, "value": group.level.get() }))
        .collect();
    let value = |key: &str| settings.get(key).unwrap_or(Value::Null);
    let mut form = Map::new();
    form.insert(
        "Options".to_owned(),
        json!([
            {
                "type": "select",
                "name": "CHAT-Auth-include",
                "label": "Minimum Authentication",
                "value": value("CHAT-Auth-include"),
                "options": groups,
            },
            {
                "type": "number",
                "name": "CHAT-messageLoadLimit",
                "label": "# of Previous Messages",
                "value": value("CHAT-messageLoadLimit"),
                "placeholder": "",
            },
        ]),
    );
    Ok(Envelope::success(form))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sender_may_send_ten_messages_in_thirty_seconds() {
        let hub = ChatHub::default();
        let start = Instant::now();
        for _ in 0..MOST_SENT {
            assert!(hub.may_send("user 1", start));
        }
        assert!(!hub.may_send("user 1", start));
        assert!(hub.may_send("user 2", start), "each sender counts alone");
        assert!(hub.may_send("user 1", start + SEND_WINDOW));
    }

    #[test]
    fn a_guest_never_reads_the_chat() {
        let mut settings = Settings::defaults();
        settings.chat_auth_include = 999;
        assert_eq!(reader_level(&settings), GroupLevel::LOGGED_IN);
        settings.chat_auth_include = 4;
        assert_eq!(reader_level(&settings).get(), 4);
    }
}
