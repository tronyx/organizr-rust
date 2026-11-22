//! Typed registry of every Organizr setting: type, default, group, minimum level and form
//! descriptor.
//!
//! [`Settings`] holds every setting as a typed field. The stored form of each value is the
//! one the PHP application uses, so the API can return settings as it always has.

mod choices;
mod codec;
mod registry;

pub use crate::choices::{AuthType, UnsortedTabs};
pub use crate::codec::{Choice, Codec, Flag, List, Number, Secret, Text, TextChoice, TextNumber};
pub use crate::registry::{
    REMOVED_SETTING_PREFIXES, RENAMED_SETTINGS, Settings, SettingsError, is_removed_setting,
};
