//! Groups and their access levels.

use thiserror::Error;

use crate::{Change, GroupId};

/// The access level of a group. A lower number means more access.
///
/// The API calls this `group_id`. Admin is 0, a guest is 999, and a route asks for a level
/// that the level of the caller must not exceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GroupLevel(u16);

impl GroupLevel {
    /// The Admin group: full access.
    pub const ADMIN: Self = Self(0);

    /// The Co-Admin group: the level that the routes for managing the install ask for.
    pub const CO_ADMIN: Self = Self(1);

    /// The Guest group: a caller who is not logged in.
    pub const GUEST: Self = Self(999);

    /// The level that every logged-in user has or passes. A route for this level is for
    /// everybody but guests.
    pub const LOGGED_IN: Self = Self(998);

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

    /// The level for a new group, when this is the highest level in use below Guest:
    /// the next one. There is none when the next one is the level of Guest.
    #[must_use]
    pub fn next_for_a_new_group(self) -> Option<Self> {
        let next = self.0.checked_add(1)?;
        (next < Self::GUEST.0).then_some(Self(next))
    }

    /// Whether the level is one of the two that every install has and keeps: Admin
    /// and Guest. Their groups cannot be deleted, and neither can be the default group.
    #[must_use]
    pub fn is_fixed(self) -> bool {
        self == Self::ADMIN || self == Self::GUEST
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

/// Changes to a group. A field that is [`Change::Keep`] stays as it is. The level of a
/// group never changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupChanges {
    /// The name.
    pub name: Change<String>,
    /// The picture.
    pub image: Change<String>,
    /// Whether new users join this group.
    pub is_default: Change<bool>,
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
    fn new_group_gets_the_level_after_the_highest_one_below_guest() {
        let level = |value| GroupLevel::new(value).expect("the level is valid");

        assert_eq!(level(4).next_for_a_new_group(), Some(level(5)));
        assert_eq!(level(997).next_for_a_new_group(), Some(level(998)));
        // The next level would be the one of Guest.
        assert_eq!(level(998).next_for_a_new_group(), None);
        assert_eq!(GroupLevel::GUEST.next_for_a_new_group(), None);
    }

    #[test]
    fn admin_and_guest_are_the_fixed_levels() {
        assert!(GroupLevel::ADMIN.is_fixed());
        assert!(GroupLevel::GUEST.is_fixed());
        assert!(!GroupLevel::CO_ADMIN.is_fixed());
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
