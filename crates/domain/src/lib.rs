//! Domain types for Organizr: ids, levels, enums, state machines and domain errors. No I/O.

mod bookmark;
mod category;
mod change;
mod chat;
pub mod email;
mod group;
mod ids;
mod invite;
mod menu_name;
mod session;
mod tab;
mod time;
mod user;

pub use bookmark::{
    Bookmark, BookmarkCategory, BookmarkCategoryChanges, BookmarkChanges, NewBookmark,
    NewBookmarkCategory, is_hex_color,
};
pub use category::{Category, CategoryChanges, NewCategory};
pub use change::Change;
pub use chat::ChatMessage;
pub use group::{Group, GroupChanges, GroupLevel, InvalidGroupLevel, NewGroup};
pub use ids::{CategoryId, CategoryNumber, GroupId, SessionId, TabId, UserId};
pub use invite::{Invite, NewInvite};
pub use menu_name::{InvalidMenuName, MENU_NAME_LIMIT, menu_name};
pub use session::{NewSession, Session, SessionTokenHash};
pub use tab::{InvalidLevelRange, InvalidTabKind, LevelRange, NewTab, Tab, TabChanges, TabKind};
pub use time::Timestamp;
pub use user::{NewUser, PasswordHash, User, UserChanges};
