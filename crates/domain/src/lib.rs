//! Domain types for Organizr: ids, levels, enums, state machines and domain errors. No I/O.

mod category;
mod change;
pub mod email;
mod group;
mod ids;
mod session;
mod tab;
mod time;
mod user;

pub use category::{Category, NewCategory};
pub use change::Change;
pub use group::{Group, GroupLevel, InvalidGroupLevel, NewGroup};
pub use ids::{CategoryId, CategoryNumber, GroupId, SessionId, TabId, UserId};
pub use session::{NewSession, Session, SessionTokenHash};
pub use tab::{
    InvalidLevelRange, InvalidTabKind, InvalidTabName, LevelRange, NewTab, TAB_NAME_LIMIT, Tab,
    TabChanges, TabKind, tab_name,
};
pub use time::Timestamp;
pub use user::{NewUser, PasswordHash, User};
