//! Domain types for Organizr: ids, levels, enums, state machines and domain errors. No I/O.

mod category;
mod change;
pub mod email;
mod group;
mod ids;
mod menu_name;
mod session;
mod tab;
mod time;
mod user;

pub use category::{Category, CategoryChanges, NewCategory};
pub use change::Change;
pub use group::{Group, GroupChanges, GroupLevel, InvalidGroupLevel, NewGroup};
pub use ids::{CategoryId, CategoryNumber, GroupId, SessionId, TabId, UserId};
pub use menu_name::{InvalidMenuName, MENU_NAME_LIMIT, menu_name};
pub use session::{NewSession, Session, SessionTokenHash};
pub use tab::{InvalidLevelRange, InvalidTabKind, LevelRange, NewTab, Tab, TabChanges, TabKind};
pub use time::Timestamp;
pub use user::{NewUser, PasswordHash, User};
