//! `GET /icon`: the icons that a tab, a category or a group can have as its picture, a
//! page at a time, for an Admin or a Co-Admin.
//!
//! The list is the one of PHP Organizr, in the form of the picker of its interface:
//! families of icons, each with its icons. An icon has the stored value as `id`, for
//! example `fontawesome::cog`, and a name to show as `text`.

use std::sync::LazyLock;

use axum::Extension;
use axum::extract::RawQuery;
use organizr_domain::GroupLevel;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;

/// One icon of the list.
#[derive(Debug, Deserialize, Serialize)]
pub struct Icon {
    id: String,
    text: String,
}

/// One family of icons.
#[derive(Debug, Deserialize)]
struct Family {
    children: Vec<Icon>,
}

/// Every icon, in the order of the list.
static ICONS: LazyLock<Vec<Icon>> = LazyLock::new(|| {
    serde_json::from_str::<Vec<Family>>(include_str!("icons.json"))
        .inspect_err(|error| tracing::error!(%error, "the icon list cannot be read"))
        .unwrap_or_default()
        .into_iter()
        .flat_map(|family| family.children)
        .collect()
});

/// How many icons a page has when the query names no number.
const DEFAULT_LIMIT: u64 = 20;

/// Whether there are more pages.
#[derive(Debug, Serialize)]
pub struct Pagination {
    more: bool,
}

/// One page of icons. `limit` and `page` are sent as the query has them: as text when
/// the query names them, as a number when it does not.
#[derive(Debug, Serialize)]
pub struct IconPage {
    results: Vec<&'static Icon>,
    limit: Value,
    page: Value,
    term: Option<String>,
    total: usize,
    pagination: Pagination,
}

/// A number of the query: the text and its value. It must be a whole number above 0.
fn number(given: Option<String>, default: u64) -> Result<(Value, u64), Refusal> {
    let Some(text) = given else {
        return Ok((Value::from(default), default));
    };
    match text.trim().parse::<u64>() {
        Ok(value) if value > 0 => Ok((Value::from(text), value)),
        _ => Err(Refusal::unprocessable(
            "Page and limit must be whole numbers above 0",
        )),
    }
}

/// `GET /icon`: the icons whose name holds the text `search`, in any letter case, or
/// all icons when `search` is empty or `0`. `page` starts at 1, and `limit` is the
/// number of icons on a page.
pub async fn list(
    Extension(caller): Extension<Caller>,
    RawQuery(query): RawQuery,
) -> Result<Envelope<IconPage>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let (mut term, mut page, mut limit) = (None, None, None);
    for (key, value) in form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        match key.as_ref() {
            "search" => term = Some(value.into_owned()),
            "page" => page = Some(value.into_owned()),
            "limit" => limit = Some(value.into_owned()),
            _ => {}
        }
    }
    let (page_text, page) = number(page, 1)?;
    let (limit_text, limit) = number(limit, DEFAULT_LIMIT)?;

    // PHP takes the empty text and `0` for "no search".
    let wanted = term
        .as_deref()
        .filter(|term| !term.is_empty() && *term != "0")
        .map(str::to_ascii_lowercase);
    let found: Vec<&'static Icon> = ICONS
        .iter()
        .filter(|icon| {
            wanted
                .as_deref()
                .is_none_or(|wanted| icon.text.to_ascii_lowercase().contains(wanted))
        })
        .collect();
    let total = found.len();
    let skip = usize::try_from(page.saturating_sub(1).saturating_mul(limit)).unwrap_or(usize::MAX);
    let take = usize::try_from(limit).unwrap_or(usize::MAX);
    let pages = (u64::try_from(total).unwrap_or(u64::MAX)).div_ceil(limit);
    Ok(Envelope::success(IconPage {
        results: found.into_iter().skip(skip).take(take).collect(),
        limit: limit_text,
        page: page_text,
        term,
        total,
        pagination: Pagination { more: page < pages },
    }))
}

#[cfg(test)]
mod tests {
    use super::ICONS;

    #[test]
    fn list_holds_the_icons_of_php_organizr() {
        assert_eq!(ICONS.len(), 36 + 675 + 351 + 1836 + 192 + 234);
        assert_eq!(
            ICONS.first().map(|icon| icon.id.as_str()),
            Some("alphanumeric::A")
        );
        assert!(ICONS.iter().any(|icon| icon.id == "fontawesome::cog"));
    }
}
