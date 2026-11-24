//! How the page looks for one person: the layout, the theme and the color set.
//!
//! The Admin sets the defaults of the install. A person may choose their own look, which
//! is stored per layout (D-125). This module knows the layouts and themes that exist and
//! works out the look that applies.

use organizr_db::StoredLook;
use organizr_settings::Settings;
use serde::Serialize;

/// The layouts: the original one and the modern one.
pub const LAYOUTS: [&str; 2] = ["og", "modern"];

/// The color styles of the original layout.
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

/// The themes of the modern layout, each with its color sets. The first color set of a
/// theme is the one used when nothing else says which.
pub const MODERN_THEMES: &[(&str, &[&str])] = &[
    ("organizr", &["dark", "light"]),
    ("plex", &["dark", "light"]),
    ("jellyfin", &["dark", "light"]),
    ("emby", &["dark", "light"]),
    ("silo", &["ember", "ice", "rose"]),
    ("nocturne", &["amethyst", "crimson", "moonlit"]),
    ("riot", &["acid", "hotwire", "blacklight"]),
    ("tempest", &["squall", "abyss", "gale"]),
    ("circuit", &["matrix", "synth", "amber"]),
    ("prism", &["daydream", "twilight", "candy"]),
];

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
    /// The color set of the theme: the style of the original layout, or a color set of
    /// a modern theme.
    pub color_set: String,
    /// Whether the person chose this look.
    pub source: LookSource,
}

/// The color sets of a modern theme, or `None` when there is no such theme.
pub fn modern_color_sets(theme: &str) -> Option<&'static [&'static str]> {
    MODERN_THEMES
        .iter()
        .find(|(name, _)| *name == theme)
        .map(|(_, sets)| *sets)
}

/// The modern theme and color set of the Admin. A color set that the theme does not
/// have gives the first set of the theme, as for a person; an unknown theme gives
/// `organizr` and `dark`.
fn admin_modern(settings: &Settings) -> (String, String) {
    let theme = settings.modern_theme.as_str();
    let set = settings.modern_color_set.as_str();
    match modern_color_sets(theme) {
        Some(sets) if sets.contains(&set) => (theme.to_owned(), set.to_owned()),
        Some(sets) => (
            theme.to_owned(),
            sets.first().map_or("dark", |first| first).to_owned(),
        ),
        None => ("organizr".to_owned(), "dark".to_owned()),
    }
}

/// The modern theme that applies when a person chose none.
pub fn default_modern_theme(settings: &Settings) -> String {
    admin_modern(settings).0
}

/// The color set that a modern theme has when a person chose none: the one of the
/// Admin if the Admin's theme is this theme, else the first one of the theme.
pub fn default_modern_color_set(settings: &Settings, theme: &str) -> String {
    let (admin_theme, admin_set) = admin_modern(settings);
    if admin_theme == theme {
        return admin_set;
    }
    modern_color_sets(theme)
        .and_then(|sets| sets.first())
        .map_or_else(|| "dark".to_owned(), |set| (*set).to_owned())
}

/// A stored text, when it is one of the allowed values.
fn known<'a>(value: Option<&'a String>, allowed: &[&str]) -> Option<&'a str> {
    value
        .map(String::as_str)
        .filter(|text| allowed.contains(text))
}

/// The theme, the color set and whether the person chose one of them, for the original
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

/// The theme, the color set and whether the person chose one of them, for the modern
/// layout.
fn modern_look(settings: &Settings, stored: Option<&StoredLook>) -> (String, String, bool) {
    let stored_theme = stored.and_then(|s| s.modern_theme.as_ref());
    let names: Vec<&str> = MODERN_THEMES.iter().map(|(name, _)| *name).collect();
    let theme = known(stored_theme, &names);
    if stored_theme.is_some() && theme.is_none() {
        // A theme that does not exist: the color set that goes with it is no use.
        let (theme, set) = admin_modern(settings);
        return (theme, set, false);
    }
    let theme_name = theme.map_or_else(|| admin_modern(settings).0, str::to_owned);
    let set = known(
        stored.and_then(|s| s.modern_color_set.as_ref()),
        modern_color_sets(&theme_name).unwrap_or_default(),
    );
    let color_set = set.map_or_else(
        || default_modern_color_set(settings, &theme_name),
        str::to_owned,
    );
    (theme_name, color_set, theme.is_some() || set.is_some())
}

/// The look that applies to a person, from the defaults of the install and what the
/// person stored. A stored value that is not valid is ignored.
pub fn effective_look(settings: &Settings, stored: Option<&StoredLook>) -> Look {
    let own_layout = known(stored.and_then(|s| s.layout.as_ref()), &LAYOUTS);
    let layout = own_layout
        .or_else(|| known(Some(&settings.layout), &LAYOUTS))
        .unwrap_or("og");
    let (theme, color_set, chosen) = if layout == "modern" {
        modern_look(settings, stored)
    } else {
        og_look(settings, stored)
    };
    Look {
        layout: layout.to_owned(),
        theme,
        color_set,
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
        (&look.layout, &look.theme, &look.color_set)
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
            s.modern_color_set = Some(text("light"));
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
            s.modern_color_set = Some(text("purple"));
        });
        let good = stored(|s| {
            s.layout = Some(text("modern"));
            s.modern_theme = Some(text("organizr"));
            s.modern_color_set = Some(text("dark"));
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
    fn light_color_set_is_honored() {
        // A stored or Admin `light` gives the light colors of the modern theme.
        let mut settings = Settings::defaults();
        settings.modern_color_set = "light".to_owned();
        let own = stored(|s| {
            s.layout = Some(text("modern"));
            s.modern_theme = Some(text("organizr"));
            s.modern_color_set = Some(text("light"));
        });

        assert_eq!(
            parts(&effective_look(&settings, Some(&own))),
            ("modern", "organizr", "light")
        );
        settings.layout = "modern".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "organizr", "light")
        );
        // A person who chose dark keeps it over the Admin's light.
        let dark = stored(|s| {
            s.layout = Some(text("modern"));
            s.modern_theme = Some(text("organizr"));
            s.modern_color_set = Some(text("dark"));
        });
        assert_eq!(
            parts(&effective_look(&settings, Some(&dark))),
            ("modern", "organizr", "dark")
        );
    }

    #[test]
    fn lists_the_modern_themes() {
        let sets = &["dark", "light"][..];
        assert_eq!(
            MODERN_THEMES,
            &[
                ("organizr", sets),
                ("plex", sets),
                ("jellyfin", sets),
                ("emby", sets),
                ("silo", &["ember", "ice", "rose"][..]),
                ("nocturne", &["amethyst", "crimson", "moonlit"][..]),
                ("riot", &["acid", "hotwire", "blacklight"][..]),
                ("tempest", &["squall", "abyss", "gale"][..]),
                ("circuit", &["matrix", "synth", "amber"][..]),
                ("prism", &["daydream", "twilight", "candy"][..]),
            ]
        );
    }

    #[test]
    fn tempest_has_its_own_color_sets() {
        let mut settings = Settings::defaults();
        let tempest = |set: Option<&str>| {
            stored(|s| {
                s.layout = Some(text("modern"));
                s.modern_theme = Some(text("tempest"));
                s.modern_color_set = set.map(text);
            })
        };
        for set in ["squall", "abyss", "gale"] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&tempest(Some(set))))),
                ("modern", "tempest", set)
            );
        }
        // A set that Tempest does not have, and no set at all: its first set.
        for set in [Some("dark"), Some("acid"), None] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&tempest(set)))),
                ("modern", "tempest", "squall")
            );
        }
        // The same for the Admin's default.
        settings.layout = "modern".to_owned();
        settings.modern_theme = "tempest".to_owned();
        settings.modern_color_set = "hotwire".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "tempest", "squall")
        );
        settings.modern_color_set = "gale".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "tempest", "gale")
        );
    }

    #[test]
    fn prism_has_its_own_color_sets() {
        let mut settings = Settings::defaults();
        let prism = |set: Option<&str>| {
            stored(|s| {
                s.layout = Some(text("modern"));
                s.modern_theme = Some(text("prism"));
                s.modern_color_set = set.map(text);
            })
        };
        for set in ["daydream", "twilight", "candy"] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&prism(Some(set))))),
                ("modern", "prism", set)
            );
        }
        // A set that Prism does not have, and no set at all: its first set.
        for set in [Some("light"), Some("matrix"), None] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&prism(set)))),
                ("modern", "prism", "daydream")
            );
        }
        // The same for the Admin's default.
        settings.layout = "modern".to_owned();
        settings.modern_theme = "prism".to_owned();
        settings.modern_color_set = "light".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "prism", "daydream")
        );
        settings.modern_color_set = "candy".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "prism", "candy")
        );
    }

    #[test]
    fn circuit_has_its_own_color_sets() {
        let mut settings = Settings::defaults();
        let circuit = |set: Option<&str>| {
            stored(|s| {
                s.layout = Some(text("modern"));
                s.modern_theme = Some(text("circuit"));
                s.modern_color_set = set.map(text);
            })
        };
        for set in ["matrix", "synth", "amber"] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&circuit(Some(set))))),
                ("modern", "circuit", set)
            );
        }
        // A set that Circuit does not have, and no set at all: its first set.
        for set in [Some("dark"), Some("acid"), None] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&circuit(set)))),
                ("modern", "circuit", "matrix")
            );
        }
        // The same for the Admin's default.
        settings.layout = "modern".to_owned();
        settings.modern_theme = "circuit".to_owned();
        settings.modern_color_set = "hotwire".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "circuit", "matrix")
        );
        settings.modern_color_set = "amber".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "circuit", "amber")
        );
    }

    #[test]
    fn riot_has_its_own_color_sets() {
        let mut settings = Settings::defaults();
        let riot = |set: Option<&str>| {
            stored(|s| {
                s.layout = Some(text("modern"));
                s.modern_theme = Some(text("riot"));
                s.modern_color_set = set.map(text);
            })
        };
        for set in ["acid", "hotwire", "blacklight"] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&riot(Some(set))))),
                ("modern", "riot", set)
            );
        }
        // A set that Riot does not have, and no set at all: its first set.
        for set in [Some("dark"), Some("amethyst"), None] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&riot(set)))),
                ("modern", "riot", "acid")
            );
        }
        // The same for the Admin's default.
        settings.layout = "modern".to_owned();
        settings.modern_theme = "riot".to_owned();
        settings.modern_color_set = "moonlit".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "riot", "acid")
        );
        settings.modern_color_set = "blacklight".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "riot", "blacklight")
        );
    }

    #[test]
    fn nocturne_has_its_own_color_sets() {
        let mut settings = Settings::defaults();
        let nocturne = |set: Option<&str>| {
            stored(|s| {
                s.layout = Some(text("modern"));
                s.modern_theme = Some(text("nocturne"));
                s.modern_color_set = set.map(text);
            })
        };
        for set in ["amethyst", "crimson", "moonlit"] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&nocturne(Some(set))))),
                ("modern", "nocturne", set)
            );
        }
        // A set that Nocturne does not have, and no set at all: its first set.
        for set in [Some("dark"), Some("ember"), None] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&nocturne(set)))),
                ("modern", "nocturne", "amethyst")
            );
        }
        // The same for the Admin's default.
        settings.layout = "modern".to_owned();
        settings.modern_theme = "nocturne".to_owned();
        settings.modern_color_set = "light".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "nocturne", "amethyst")
        );
        settings.modern_color_set = "moonlit".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "nocturne", "moonlit")
        );
    }

    #[test]
    fn silo_has_its_own_color_sets() {
        let mut settings = Settings::defaults();
        let silo = |set: Option<&str>| {
            stored(|s| {
                s.layout = Some(text("modern"));
                s.modern_theme = Some(text("silo"));
                s.modern_color_set = set.map(text);
            })
        };
        for set in ["ember", "ice", "rose"] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&silo(Some(set))))),
                ("modern", "silo", set)
            );
        }
        // `dark` is not a set of Silo, and no set at all: the first set of Silo.
        for set in [Some("dark"), None] {
            assert_eq!(
                parts(&effective_look(&settings, Some(&silo(set)))),
                ("modern", "silo", "ember")
            );
        }
        // The Admin's Silo with a set it does not have gets the first set of Silo, as
        // for a person.
        settings.layout = "modern".to_owned();
        settings.modern_theme = "silo".to_owned();
        settings.modern_color_set = "dark".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "silo", "ember")
        );
        // An unknown theme still gives the default.
        settings.modern_theme = "nonsense".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "organizr", "dark")
        );
        settings.modern_theme = "silo".to_owned();
        settings.modern_color_set = "ice".to_owned();
        assert_eq!(
            parts(&effective_look(&settings, None)),
            ("modern", "silo", "ice")
        );
    }

    #[test]
    fn a_brand_theme_is_honoured_and_its_unknown_set_is_not() {
        let settings = Settings::defaults();
        for theme in ["plex", "jellyfin", "emby"] {
            let light = stored(|s| {
                s.layout = Some(text("modern"));
                s.modern_theme = Some(text(theme));
                s.modern_color_set = Some(text("light"));
            });
            assert_eq!(
                parts(&effective_look(&settings, Some(&light))),
                ("modern", theme, "light")
            );
            let purple = stored(|s| {
                s.layout = Some(text("modern"));
                s.modern_theme = Some(text(theme));
                s.modern_color_set = Some(text("purple"));
            });
            assert_eq!(
                parts(&effective_look(&settings, Some(&purple))),
                ("modern", theme, "dark")
            );
        }
    }
}
