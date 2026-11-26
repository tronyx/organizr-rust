//! What an import tells the person who runs it.

use std::fmt;
use std::fmt::Write as _;

/// A text with its control characters written as `\u{1b}`, so that a name from the PHP
/// install cannot move the cursor or change the colors of a terminal. Other letters,
/// also of other languages, stay as they are.
pub fn escape_controls(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for letter in text.chars() {
        if letter.is_control() {
            let _ = write!(escaped, "\\u{{{:x}}}", u32::from(letter));
        } else {
            escaped.push(letter);
        }
    }
    escaped
}

/// The result of an import, or of a dry run: counts, repairs, and what was left out.
/// It holds names of records and settings, and never a secret value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Whether nothing was written.
    pub dry_run: bool,
    /// The number of groups.
    pub groups: usize,
    /// The number of users.
    pub users: usize,
    /// The number of categories.
    pub categories: usize,
    /// The number of tabs.
    pub tabs: usize,
    /// The number of bookmarks of the bookmark plugin.
    pub bookmarks: usize,
    /// The number of categories of the bookmark plugin.
    pub bookmark_categories: usize,
    /// The number of invites of the invites plugin.
    pub invites: usize,
    /// The number of messages of the chat plugin.
    pub chat_messages: usize,
    /// The number of settings that differ from the default.
    pub settings: usize,
    /// The number of sessions of the PHP install. Sessions are not imported.
    pub sessions: u64,
    /// The number of uploaded pictures that were copied.
    pub pictures: usize,
    /// The number of other files that were copied.
    pub other_files: usize,
    /// Records that were changed so that they can be stored.
    pub repairs: Vec<String>,
    /// Records that were left out.
    pub skipped: Vec<String>,
    /// Settings that this application does not know. They are kept, sealed.
    pub legacy_settings: Vec<String>,
    /// Settings of the PHP application that this application does not need.
    pub retired_settings: Vec<String>,
    /// Tables whose rows are not imported yet: name and number of rows.
    pub tables_not_imported: Vec<(String, u64)>,
    /// PHP files of the install outside the PHP application itself. They are never run.
    pub php_files: Vec<String>,
    /// Other things to look at after the import.
    pub notes: Vec<String>,
    /// The changes of this version that apply to this install: what worked in PHP
    /// Organizr and works differently now.
    pub breaking: Vec<String>,
}

impl fmt::Display for Report {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (title, verb) = if self.dry_run {
            ("Dry run: nothing was written.", "would be imported")
        } else {
            ("The import is complete.", "were imported")
        };
        writeln!(formatter, "{title}")?;
        writeln!(formatter)?;
        writeln!(formatter, "These records {verb}:")?;
        for (count, kind) in [
            (self.groups, "groups"),
            (self.users, "users"),
            (self.categories, "categories"),
            (self.tabs, "tabs"),
            (self.bookmark_categories, "bookmark categories"),
            (self.bookmarks, "bookmarks"),
            (self.invites, "invites"),
            (self.chat_messages, "chat messages"),
            (self.settings, "settings that differ from the default"),
            (self.pictures, "uploaded pictures"),
            (self.other_files, "other files"),
        ] {
            writeln!(formatter, "  {count:>6}  {kind}")?;
        }
        writeln!(formatter)?;
        writeln!(
            formatter,
            "Sessions are not imported ({} found). Every user logs in again, with the same password.",
            self.sessions
        )?;

        breaking(formatter, &self.breaking)?;
        list(formatter, "Repairs", &self.repairs)?;
        list(formatter, "Left out", &self.skipped)?;
        list(formatter, "To look at", &self.notes)?;
        let tables: Vec<String> = self
            .tables_not_imported
            .iter()
            .map(|(table, rows)| format!("{table}: {rows} rows"))
            .collect();
        list(
            formatter,
            "Tables that this version does not import yet (their rows stay in the old database)",
            &tables,
        )?;
        list(
            formatter,
            "Settings that this application does not know (kept, sealed, in the table legacy_settings)",
            &self.legacy_settings,
        )?;
        list(
            formatter,
            "Settings that this application does not need (not stored)",
            &self.retired_settings,
        )?;
        list(
            formatter,
            "PHP files of the old install (never run; their work needs a new home)",
            &self.php_files,
        )
    }
}

fn list(formatter: &mut fmt::Formatter<'_>, title: &str, lines: &[String]) -> fmt::Result {
    if lines.is_empty() {
        return Ok(());
    }
    writeln!(formatter)?;
    writeln!(formatter, "{title}:")?;
    for line in lines {
        writeln!(formatter, "  - {}", escape_controls(line))?;
    }
    Ok(())
}

/// The breaking changes that apply to the install, set apart from the other lists, with
/// where to read about the ones that the data of an install doesn't show.
fn breaking(formatter: &mut fmt::Formatter<'_>, changes: &[String]) -> fmt::Result {
    writeln!(formatter)?;
    if changes.is_empty() {
        writeln!(
            formatter,
            "Breaking changes for this install: none found in its data."
        )?;
    } else {
        writeln!(formatter, "!!! Breaking changes for this install !!!")?;
        writeln!(
            formatter,
            "These worked in PHP Organizr and work differently now:"
        )?;
        for change in changes {
            writeln!(formatter, "  - {}", escape_controls(change))?;
        }
    }
    writeln!(
        formatter,
        "Check your reverse proxy too: the forward auth checks changed (D-136, D-157). \"Moving from PHP Organizr\" in the docs lists every breaking change."
    )
}

#[cfg(test)]
mod tests {
    use super::Report;

    #[test]
    fn control_characters_in_the_lines_are_escaped_and_other_letters_kept() {
        let report = Report {
            repairs: vec!["The tab \"\u{1b}]0;evil\u{7}\" was renamed.".to_owned()],
            skipped: vec!["The user \"caf\u{e9}\u{85}\u{7f}\" is left out.".to_owned()],
            notes: vec!["A line\nthat was two.".to_owned()],
            php_files: vec!["data/pages/\u{9b}31m.php".to_owned()],
            breaking: vec!["The account \u{1b}[2J is a Co-Admin.".to_owned()],
            ..Report::default()
        };
        let text = report.to_string();
        assert!(
            !text
                .chars()
                .any(|letter| letter.is_control() && letter != '\n'),
            "{text:?}"
        );
        for part in [
            r#"The tab "\u{1b}]0;evil\u{7}" was renamed."#,
            r#"The user "café\u{85}\u{7f}" is left out."#,
            r"A line\u{a}that was two.",
            r"data/pages/\u{9b}31m.php",
            r"The account \u{1b}[2J is a Co-Admin.",
        ] {
            assert!(text.contains(part), "{part:?} in {text}");
        }
    }
}
