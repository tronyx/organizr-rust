//! Messages of the chat plugin.

/// A message of the chat, as it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    /// The id of the record.
    pub id: i64,
    /// The user name of the writer.
    pub username: String,
    /// The picture of the writer: a path or an address.
    pub gravatar: String,
    /// What tells writers with the same name apart.
    pub uid: String,
    /// When the message was sent, in seconds since 1970.
    pub sent_at: i64,
    /// The address that the message came from. The API never shows it.
    pub ip: String,
    /// The message, as plain text.
    pub message: String,
}
