//! One choice of a list that a settings page offers: what the list shows, and what the
//! setting stores. The routes of the lists that come from other services send them.

use serde::Serialize;

/// One choice of a list of a settings page: what the list shows, and what is stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Choice {
    /// What the list shows.
    pub label: String,
    /// What the setting stores.
    pub value: String,
}
