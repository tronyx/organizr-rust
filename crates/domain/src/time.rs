//! Points in time.

/// A point in time as whole seconds since 1970-01-01 00:00:00 UTC.
///
/// Both database engines store this as one integer, so times compare as numbers. The
/// PHP application stored date strings in two formats and compared them as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Creates a timestamp from seconds since the Unix epoch.
    pub const fn from_unix_seconds(seconds: i64) -> Self {
        Self(seconds)
    }

    /// Seconds since the Unix epoch.
    pub const fn unix_seconds(self) -> i64 {
        self.0
    }
}
