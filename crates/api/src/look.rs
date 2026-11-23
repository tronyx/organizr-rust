//! How the page looks for one person: the layout, the theme and the colour set.
//!
//! The Admin sets the defaults of the install. A person may choose their own look, which
//! is stored per layout (D-125). This module knows the layouts and themes that exist and
//! works out the look that applies.

use organizr_db::StoredLook;
use organizr_settings::Settings;
use serde::Serialize;

/// The layouts: the original one and the modern one.
pub const LAYOUTS: [&str; 2] = ["og", "modern"];

/// The colour styles of the original layout.
pub const OG_STYLES: [&str; 2] = ["dark", "light"];

/// The themes of the original layout.
pub const OG_THEMES: [&str; 13] = [
    "Organizr",
    "Blue",
    "Nord",
    "Dracula",
    "Overseerr",
    "Onedark",
    "Aquamarine",
    "Space_Gray",
    "Plex_theme_park",
    "Dark_theme_park",
    "Hotline",
    "Hotpink",
    "Plex",
];

/// The themes of the modern layout, each with its colour sets. The first colour set of a
/// theme is the one used when nothing else says which. The light set of `organizr` stays
/// out until the inner pages have modern styling (phase 2).
pub const MODERN_THEMES: &[(&str, &[&str])] = &[("organizr", &["dark"])];

/// Where an effective look comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LookSource {
    /// The person chose at least one part of it.
    User,
    /// The defaults of the install.
    Default,
}

/// The look that applies to one person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Look {
    /// `og` or `modern`.
    pub layout: String,
    /// The theme of that layout.
    pub theme: String,
    /// The colour set of the theme: the style of the original layout, or a colour set of
    /// a modern theme.
    pub colour_set: String,
    /// Whether the person chose this look.
    pub source: LookSource,
}

/// The colour sets of a modern theme, or `None` when there is no such theme.
pub fn modern_colour_sets(theme: &str) -> Option<&'static [&'static str]> {
    MODERN_THEMES
        .iter()
        .find(|(name, _)| *name == theme)
        .map(|(_, sets)| *sets)
}

/// The modern theme and colour set of the Admin when they exist together, else
/// `organizr` and `dark`.
fn admin_modern(settings: &Settings) -> (String, String) {
    let theme = settings.modern_theme.as_str();
    let set = settings.modern_colour_set.as_str();
    if modern_colour_sets(theme).is_some_and(|sets| sets.contains(&set)) {
        (theme.to_owned(), set.to_owned())
    } else {
        ("organizr".to_owned(), "dark".to_owned())
    }
}

/// The modern theme that applies when a person chose none.
pub fn default_modern_theme(settings: &Settings) -> String {
    admin_modern(settings).0
}

/// The colour set that a modern theme has when a person chose none: the one of the
/// Admin if the Admin's theme is this theme, else the first one of the theme.
pub fn default_modern_colour_set(settings: &Settings, theme: &str) -> String {
    let (admin_theme, admin_set) = admin_modern(settings);
    if admin_theme == theme {
        return admin_set;
    }
    modern_colour_sets(theme)
        .and_then(|sets| sets.first())
        .map_or_else(|| "dark".to_owned(), |set| (*set).to_owned())
}

/// A stored text, when it is one of the allowed values.
fn known<'a>(value: Option<&'a String>, allowed: &[&str]) -> Option<&'a str> {
    value
        .map(String::as_str)
        .filter(|text| allowed.contains(text))
}

/// The theme, the colour set and whether the person chose one of them, for the original
/// layout.
fn og_look(settings: &Settings, stored: Option<&StoredLook>) -> (String, String, bool) {
    let theme = known(stored.and_then(|s| s.og_theme.as_ref()), &OG_THEMES);
    let style = known(stored.and_then(|s| s.og_style.as_ref()), &OG_STYLES);
    (
        theme.map_or_else(
            || crate::handlers::themes::applied_theme(settings),
            str::to_owned,
        ),
        style.map_or_else(|| settings.style.clone(), str::to_owned),
        theme.is_some() || style.is_some(),
    )
}

/// The theme, the colour set and whether the person chose one of them, for the modern
/// layout.
fn modern_look(settings: &Settings, stored: Option<&StoredLook>) -> (String, String, bool) {
    let stored_theme = stored.and_then(|s| s.modern_theme.as_ref());
    let names: Vec<&str> = MODERN_THEMES.iter().map(|(name, _)| *name).collect();
    let theme = known(stored_theme, &names);
    if stored_theme.is_some() && theme.is_none() {
        // A theme that does not exist: the colour set that goes with it is no use.
        let (theme, set) = admin_modern(settings);
        return (theme, set, false);
    }
    let theme_name = theme.map_or_else(|| admin_modern(settings).0, str::to_owned);
    let set = known(
        stored.and_then(|s| s.modern_colour_set.as_ref()),
        modern_colour_sets(&theme_name).unwrap_or_default(),
    );
    let colour_set = set.map_or_else(
        || default_modern_colour_set(settings, &theme_name),
        str::to_owned,
    );
    (theme_name, colour_set, theme.is_some() || set.is_some())
}

/// The look that applies to a person, from the defaults of the install and what the
/// person stored. A stored value that is not valid is ignored.
pub fn effective_look(settings: &Settings, stored: Option<&StoredLook>) -> Look {
    let own_layout = known(stored.and_then(|s| s.layout.as_ref()), &LAYOUTS);
    let layout = own_layout
        .or_else(|| known(Some(&settings.layout), &LAYOUTS))
        .unwrap_or("og");
    let (theme, colour_set, chosen) = if layout == "modern" {
        modern_look(settings, stored)
    } else {
        og_look(settings, stored)
    };
    Look {
        layout: layout.to_owned(),
        theme,
        colour_set,
        source: if own_layout.is_some() || chosen {
            LookSource::User
        } else {
            LookSource::Default
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(fill: impl FnOnce(&mut StoredLook)) -> StoredLook {
        let mut look = StoredLook::default();
        fill(&mut look);
        look
    }

    fn text(value: &str) -> String {
        value.to_owned()
    }

    fn parts(look: &Look) -> (&str, &str, &str) {
        (&look.layout, &look.theme, &look.colour_set)
    }

    #[test]
    fn default_install_is_og_with_the_admin_theme() {
        let mut settings = Settings::defaults();
        settings.style = "light".to_owned();

        let look = effective_look(&settings, None);

        assert_eq!(parts(&look), ("og", "Organizr", "light"));
        assert_eq!(look.source, LookSource::Default);
    }

    #[test]
    fn admin_default_modern_gives_organizr_dark() {
        let mut settings = Settings::defaults();
        settings.layout = "modern".to_owned();

        let look = effective_look(&settings, None);

        assert_eq!(parts(&look), ("modern", "organizr", "dark"));
        assert_eq!(look.source, LookSource::Default);
    }

    #[test]
    fn unknown_stored_theme_falls_back_to_the_default() {
        let settings = Settings::defaults();
        let og = stored(|s| {
            s.layout = Some(text("og"));
            s.og_theme = Some(text("Nope"));
            s.og_style = Some(text("neon"));
        });
        let modern = stored(|s| {
            s.layout = Some(text("modern"));
            s.modern_theme = Some(text("nope"));
            s.modern_colour_set = Some(text("light"));
        });

        assert_eq!(
            parts(&effective_look(&settings, Some(&og))),
            ("og", "Organizr", "dark")
        );
        assert_eq!(
            parts(&effective_look(&settings, Some(&modern))),
            ("modern", "organizr", "dark")
        );
    }

    #[test]
    fn stored_og_choice_wins_over_the_admin_theme() {
        let mut settings = Settings::defaults();
        settings.layout = "modern".to_owned();
        let own = stored(|s| {
            s.layout = Some(text("og"));
            s.og_theme = Some(text("Dracula"));
            s.og_style = Some(text("light"));
        });

        let look = effective_look(&settings, Some(&own));

        assert_eq!(parts(&look), ("og", "Dracula", "light"));
        assert_eq!(look.source, LookSource::User);
    }

    #[test]
    fn stored_modern_pair_must_exist_together() {
        let settings = Settings::defaults();
        let bad = stored(|s| {
            s.layout = Some(text("modern"));
            s.modern_theme = Some(text("organizr"));
            s.modern_colour_set = Some(text("purple"));
        });
        let good = stored(|s| {
            s.layout = Some(text("modern"));
            s.modern_theme = Some(text("organizr"));
            s.modern_colour_set = Some(text("dark"));
        });

        assert_eq!(
            parts(&effective_look(&settings, Some(&bad))),
            ("modern", "organizr", "dark")
        );
        assert_eq!(
            parts(&effective_look(&settings, Some(&good))),
            ("modern", "organizr", "dark")
        );
    }

    #[test]
    fn light_colour_set_falls_back_to_dark() {
        // The light set is not offered yet: a stored or Admin `light` gives dark.
        let mut settings = Settings::defaults();
        settings.modern_colour_set = "light".to_owned();
        let own = stored(|s| {
            s.layout = Some(text("modern"));
            s.modern_theme = Some(text("organizr"));
            s.modern_colour_set = Some(text("light"));
        });

        assert_eq!(
            parts(&effective_look(&settings, Some(&own))),
            ("modern", "organizr", "dark")
        );
        settings.layout = "modern".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "organizr", "dark")
        );
    }
}
