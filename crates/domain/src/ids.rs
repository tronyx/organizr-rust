//! Identifiers. Each kind of record has its own type, so one cannot be passed for another.

/// Defines an identifier type around a whole number.
macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(i64);

        impl $name {
            /// Wraps a number.
            pub const fn new(value: i64) -> Self {
                Self(value)
            }

            /// The number.
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

id_type!(
    /// Identifies a category. The API calls it `id`.
    ///
    /// This is not the number that tabs and the API use to name a category. See
    /// [`CategoryNumber`].
    CategoryId
);

id_type!(
    /// The number that names a category in the API, where it is called `category_id`.
    ///
    /// The set-up wizard gives the first category the number 0. A new category gets the
    /// highest number plus one.
    CategoryNumber
);

id_type!(
    /// Identifies a tab.
    TabId
);
