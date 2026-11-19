//! A change to one field of a record.

/// What a change does to one field of a record.
///
/// A request that changes a record names only some of its fields. This says for one
/// field whether the request named it. For a field that can be empty, the value is an
/// `Option`, so "keep" and "make empty" stay apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Change<T> {
    /// The field stays as it is.
    #[default]
    Keep,
    /// The field gets this value.
    Set(T),
}

impl<T> Change<T> {
    /// The new value, when there is one.
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Keep => None,
            Self::Set(value) => Some(value),
        }
    }

    /// The new value, or `current` when the field stays as it is.
    pub fn or(self, current: T) -> T {
        self.into_option().unwrap_or(current)
    }
}

#[cfg(test)]
mod tests {
    use super::Change;

    #[test]
    fn change_is_keep_unless_a_value_is_set() {
        assert_eq!(Change::<i64>::default(), Change::Keep);
        assert_eq!(Change::<i64>::Keep.into_option(), None);
        assert_eq!(Change::Set(5).into_option(), Some(5));
    }

    #[test]
    fn field_gets_the_new_value_or_keeps_the_current_one() {
        assert_eq!(Change::Keep.or(1), 1);
        assert_eq!(Change::Set(5).or(1), 5);
        // "Make empty" is a value of its own.
        assert_eq!(Change::Set(None).or(Some(1)), None);
    }
}
