//! Storage of the messages of the chat plugin.

use organizr_domain::ChatMessage;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, DatabaseConnection, EntityTrait, QueryOrder,
    QuerySelect,
};

use crate::DbError;
use crate::entity::chat_message;

/// Reads and writes the messages of the chat.
#[derive(Debug, Clone, Copy)]
pub struct Chat<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Chat<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// The newest messages, at most `limit`, the newest first.
    pub async fn newest(&self, limit: u64) -> Result<Vec<ChatMessage>, DbError> {
        let rows = chat_message::Entity::find()
            .order_by_desc(chat_message::Column::SentAt)
            .order_by_desc(chat_message::Column::Id)
            .limit(limit)
            .all(self.connection)
            .await?;
        Ok(rows.into_iter().map(ChatMessage::from).collect())
    }

    /// Stores a new message. Its id is left out and comes back set.
    pub async fn insert(&self, message: ChatMessage) -> Result<ChatMessage, DbError> {
        insert_in(self.connection, message, false).await
    }
}

/// Stores a message through any connection. With `keep_id`, the record keeps its id,
/// which an import needs.
pub(crate) async fn insert_in<C: ConnectionTrait>(
    connection: &C,
    message: ChatMessage,
    keep_id: bool,
) -> Result<ChatMessage, DbError> {
    let row = chat_message::ActiveModel {
        id: if keep_id {
            ActiveValue::Set(message.id)
        } else {
            ActiveValue::NotSet
        },
        username: ActiveValue::Set(message.username),
        gravatar: ActiveValue::Set(message.gravatar),
        uid: ActiveValue::Set(message.uid),
        sent_at: ActiveValue::Set(message.sent_at),
        ip: ActiveValue::Set(message.ip),
        message: ActiveValue::Set(message.message),
    };
    Ok(row.insert(connection).await?.into())
}

impl From<chat_message::Model> for ChatMessage {
    fn from(row: chat_message::Model) -> Self {
        Self {
            id: row.id,
            username: row.username,
            gravatar: row.gravatar,
            uid: row.uid,
            sent_at: row.sent_at,
            ip: row.ip,
            message: row.message,
        }
    }
}
