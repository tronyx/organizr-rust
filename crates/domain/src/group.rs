//! Groups and their access levels.

use thiserror::Error;

use crate::GroupId;

/// The access level of a group. A lower number means more access.
///
/// The API calls this `group_id`. Admin is 0, a guest is 999, and a route asks for a level
/// that the level of the caller must not exceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GroupLevel(u16);

impl GroupLevel {
    /// The Admin group: full access.
    pub const ADMIN: Self = Self(0);

    /// The Guest group: a caller who is not logged in.
    pub const GUEST: Self = Self(999);

    /// Creates a level. Levels run from 0 to 999.
    pub fn new(value: u16) -> Result<Self, InvalidGroupLevel> {
        if value <= Self::GUEST.0 {
            Ok(Self(value))
        } else {
            Err(InvalidGroupLevel(i64::from(value)))
        }
    }

    /// The level as a number.
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<i64> for GroupLevel {
    type Error = InvalidGroupLevel;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        let level = u16::try_from(value).map_err(|_| InvalidGroupLevel(value))?;
        Self::new(level)
    }
}

impl From<GroupLevel> for i64 {
    fn from(level: GroupLevel) -> Self {
        Self::from(level.0)
    }
}

/// A number that is not a group level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a group level; levels run from 0 to 999")]
pub struct InvalidGroupLevel(i64);

/// A group of users.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The identifier of the group.
    pub id: GroupId,
    /// The access level of the members.
    pub level: GroupLevel,
    /// The name. No two groups share a name, whatever the letter case.
    pub name: String,
    /// The picture: a path or a URL.
    pub image: Option<String>,
    /// Whether new users join this group.
    pub is_default: bool,
}

/// The data for a group that does not exist yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGroup {
    /// The access level of the members.
    pub level: GroupLevel,
    /// The name.
    pub name: String,
    /// The picture: a path or a URL.
    pub image: Option<String>,
    /// Whether new users join this group.
    pub is_default: bool,
}

#[cfg(test)]
mod tests {
    use super::{GroupLevel, InvalidGroupLevel};

    #[test]
    fn levels_run_from_admin_to_guest() {
        assert_eq!(GroupLevel::new(0), Ok(GroupLevel::ADMIN));
        assert_eq!(GroupLevel::new(999), Ok(GroupLevel::GUEST));
        assert_eq!(GroupLevel::new(4).map(GroupLevel::get), Ok(4));
    }

    #[test]
    fn number_above_guest_is_not_a_level() {
        assert_eq!(GroupLevel::new(1000), Err(InvalidGroupLevel(1000)));
    }

    #[test]
    fn stored_number_converts_only_when_it_is_a_level() {
        assert_eq!(GroupLevel::try_from(1_i64), GroupLevel::new(1));
        assert_eq!(GroupLevel::try_from(-1_i64), Err(InvalidGroupLevel(-1)));
        assert_eq!(
            GroupLevel::try_from(70_000_i64),
            Err(InvalidGroupLevel(70_000))
        );
    }

    #[test]
    fn lower_level_sorts_first() {
        assert!(GroupLevel::ADMIN < GroupLevel::GUEST);
    }

    #[test]
    fn level_converts_back_to_the_stored_number() {
        assert_eq!(i64::from(GroupLevel::GUEST), 999);
    }
}
