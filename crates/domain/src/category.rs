//! Categories: the headings that tabs are grouped under in the menu.

use crate::{CategoryId, CategoryNumber};

/// A category of tabs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    /// The identifier of the category. The API calls it `id`.
    pub id: CategoryId,
    /// The number the API uses to name the category of a tab. The API calls it
    /// `category_id`. No two categories share a number.
    pub number: CategoryNumber,
    /// Where the category stands in the menu; lower comes first. The API calls it `order`.
    pub position: i64,
    /// The name. No two categories share a name, whatever the letter case.
    pub name: String,
    /// The picture: an icon name, a path or a URL.
    pub image: Option<String>,
    /// Whether new tabs join this category.
    pub is_default: bool,
}

/// The data for a category that does not exist yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCategory {
    /// The number the API uses to name the category of a tab.
    pub number: CategoryNumber,
    /// Where the category stands in the menu; lower comes first.
    pub position: i64,
    /// The name.
    pub name: String,
    /// The picture: an icon name, a path or a URL.
    pub image: Option<String>,
    /// Whether new tabs join this category.
    pub is_default: bool,
}
