//! Domain types for Organizr: ids, levels, enums, state machines and domain errors. No I/O.

mod category;
mod group;
mod ids;
mod tab;
mod time;
mod user;

pub use category::{Category, NewCategory};
pub use group::{Group, GroupLevel, InvalidGroupLevel, NewGroup};
pub use ids::{CategoryId, CategoryNumber, GroupId, TabId, UserId};
pub use tab::{InvalidLevelRange, InvalidTabKind, LevelRange, NewTab, Tab, TabKind};
pub use time::Timestamp;
pub use user::{NewUser, PasswordHash, User};
