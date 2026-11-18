//! Domain types for Organizr: ids, levels, enums, state machines and domain errors. No I/O.

mod group;
mod ids;
mod time;
mod user;

pub use group::{Group, GroupLevel, InvalidGroupLevel, NewGroup};
pub use ids::{GroupId, UserId};
pub use time::Timestamp;
pub use user::{NewUser, PasswordHash, User};
