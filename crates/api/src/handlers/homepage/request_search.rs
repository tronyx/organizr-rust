//! The search for new requests: the films and series that Overseerr or Ombi finds for a
//! text, in one shape for the interface. PHP Organizr asks the TMDB from the browser;
//! here the server asks the request service with its stored key (D-108).

use serde_json::{Value, json};

/// The longest text that a search takes.
pub const LONGEST_QUERY: usize = 200;

/// A text for a path or a query of a request service: every byte that is not a letter,
/// a digit or one of `-._*` as `%XX`, and a space as `%20`.
pub fn encoded(text: &str) -> String {
    form_urlencoded::byte_serialize(text.as_bytes())
        .collect::<String>()
        .replace('+', "%20")
}

/// The address of a poster of the TMDB, or the picture of PHP for an item without one.
pub fn poster(path: &str) -> String {
    match path.trim_start_matches('/') {
        "" => "plugins/images/homepage/no-list.png".to_owned(),
        path if path.starts_with("http://") || path.starts_with("https://") => path.to_owned(),
        path => format!("https://image.tmdb.org/t/p/w300/{path}"),
    }
}

/// One match: its TMDB id, `movie` or `tv`, its title, year, poster and summary, and
/// whether the service has it already (`available`) or it is requested (`requested`).
pub fn result(
    id: i64,
    kind: &str,
    (title, date): (&str, &str),
    (poster_path, overview): (&str, &str),
    status: Option<&str>,
) -> Value {
    json!({
        "id": id,
        "type": kind,
        "title": title,
        "year": date.get(..4).unwrap_or_default(),
        "poster": poster(poster_path),
        "overview": overview,
        "status": status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_is_encoded_for_a_path_and_a_query() {
        assert_eq!(encoded("The Film: 2/3+"), "The%20Film%3A%202%2F3%2B");
    }

    #[test]
    fn a_poster_is_an_address_of_the_tmdb_or_the_picture_for_none() {
        assert_eq!(poster("/a.jpg"), "https://image.tmdb.org/t/p/w300/a.jpg");
        assert_eq!(poster(""), "plugins/images/homepage/no-list.png");
        assert_eq!(
            poster("https://example.com/a.jpg"),
            "https://example.com/a.jpg"
        );
    }
}
