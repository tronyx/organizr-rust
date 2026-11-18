//! Identifiers. Each kind of record has its own type, so one cannot be passed for another.

/// Defines an identifier type around the database row id.
macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(i64);

        impl $name {
            /// Wraps a row id.
            pub const fn new(value: i64) -> Self {
                Self(value)
            }

            /// The row id.
            pub const fn get(self) -> i64 {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

id_type!(
    /// Identifies a group. The API calls it `id`.
    ///
    /// This is not the level of the group, which the API calls `group_id`. See
    /// [`GroupLevel`](crate::GroupLevel).
    GroupId
);

id_type!(
    /// Identifies a user.
    UserId
);
