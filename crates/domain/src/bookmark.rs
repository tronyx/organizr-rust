//! Bookmarks: links on a page of their own, in categories.
//!
//! The bookmark plugin has tabs and categories of its own, apart from the tabs and
//! categories of the menu.

/// A category of bookmarks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookmarkCategory {
    /// The id of the record.
    pub id: i64,
    /// The number that bookmarks name their category with. The API calls it
    /// `category_id`.
    pub number: i64,
    /// The place on the page. The API calls it `order`.
    pub position: i64,
    /// The name. The API calls it `category`.
    pub name: String,
    /// Whether new bookmarks join this category.
    pub is_default: bool,
}

/// A category of bookmarks that is not stored yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBookmarkCategory {
    /// The number.
    pub number: i64,
    /// The place on the page.
    pub position: i64,
    /// The name.
    pub name: String,
    /// Whether new bookmarks join this category.
    pub is_default: bool,
}

/// The fields of a category of bookmarks that a change sets. A field that is `None`
/// stays as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BookmarkCategoryChanges {
    /// The number.
    pub number: Option<i64>,
    /// The place on the page.
    pub position: Option<i64>,
    /// The name.
    pub name: Option<String>,
    /// Whether new bookmarks join this category.
    pub is_default: Option<bool>,
}

/// A bookmark. The API calls it a tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    /// The id of the record.
    pub id: i64,
    /// The place in its category. The API calls it `order`.
    pub position: i64,
    /// The number of its category. The API calls it `category_id`.
    pub category: i64,
    /// The name.
    pub name: String,
    /// The address that the bookmark opens.
    pub url: String,
    /// Whether the page shows the bookmark.
    pub enabled: bool,
    /// The level with the least access that sees the bookmark. The API calls it
    /// `group_id`.
    pub least_access: i32,
    /// The picture, in the form of a picture of a tab.
    pub image: String,
    /// The color of the tile: `#` and three or six hexadecimal digits.
    pub background_color: String,
    /// The color of the name.
    pub text_color: String,
}

/// A bookmark that is not stored yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBookmark {
    /// The place in its category.
    pub position: i64,
    /// The number of its category.
    pub category: i64,
    /// The name.
    pub name: String,
    /// The address.
    pub url: String,
    /// Whether the page shows the bookmark.
    pub enabled: bool,
    /// The level with the least access that sees the bookmark.
    pub least_access: i32,
    /// The picture.
    pub image: String,
    /// The color of the tile.
    pub background_color: String,
    /// The color of the name.
    pub text_color: String,
}

/// The fields of a bookmark that a change sets. A field that is `None` stays as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BookmarkChanges {
    /// The place in its category.
    pub position: Option<i64>,
    /// The number of its category.
    pub category: Option<i64>,
    /// The name.
    pub name: Option<String>,
    /// The address.
    pub url: Option<String>,
    /// Whether the page shows the bookmark.
    pub enabled: Option<bool>,
    /// The level with the least access that sees the bookmark.
    pub least_access: Option<i32>,
    /// The picture.
    pub image: Option<String>,
    /// The color of the tile.
    pub background_color: Option<String>,
    /// The color of the name.
    pub text_color: Option<String>,
}

/// Whether a text is a color as the bookmark plugin takes it: `#` and three or six
/// hexadecimal digits.
pub fn is_hex_color(text: &str) -> bool {
    text.strip_prefix('#').is_some_and(|digits| {
        (digits.len() == 3 || digits.len() == 6)
            && digits.chars().all(|digit| digit.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::is_hex_color;

    #[test]
    fn color_is_a_hash_and_three_or_six_hexadecimal_digits() {
        for color in ["#fff", "#FFFFFF", "#0a1B2c", "#000"] {
            assert!(is_hex_color(color), "{color}");
        }
        for color in [
            "fff", "#ffff", "#fffff", "#fffffff", "#ggg", "", "#", "red", "#fff;",
        ] {
            assert!(!is_hex_color(color), "{color}");
        }
    }
}
