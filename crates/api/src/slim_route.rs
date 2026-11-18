//! Expansion of the PHP application's route patterns into explicit Axum routes.
//!
//! Organizr registers its routes with Slim, which uses `FastRoute` patterns:
//!
//! - `{name}` matches one path segment
//! - `{name:.*}` matches the rest of the path, including nothing
//! - `[...]` marks an optional part; optional parts sit at the end and can be nested,
//!   as in `/auth[/[{group}[/{type}[/{ips}]]]]`
//!
//! Axum has no optional parts, and its wildcard does not match an empty rest. [`expand`]
//! turns one pattern into every explicit route it stands for, so the route table of the
//! Rust application can keep the patterns of the PHP source.
//!
//! Parameters get positional names (`{p0}`, `{p1}`). Two patterns of the same shape then
//! give the same Axum path, whatever their parameter names. Axum needs that to put
//! `GET /requests/{type}/{limit}` and `POST /requests/{type}/{id}` on one path.

use thiserror::Error;

/// One explicit route that a pattern stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedRoute {
    path: String,
    params: Vec<String>,
    empty_catch_all: Option<String>,
}

impl ExpandedRoute {
    /// The route in Axum's syntax, for example `/log/{p0}/{p1}`.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The pattern's names for the captured parameters, in the order they appear.
    pub fn params(&self) -> &[String] {
        &self.params
    }

    /// The name of a `{name:.*}` parameter that this route matches with an empty value.
    ///
    /// Axum's wildcard needs at least one character, so a pattern such as
    /// `/socks/{app}/{route:.*}` gives a second route, `/socks/{p0}/`, for the empty rest.
    pub fn empty_catch_all(&self) -> Option<&str> {
        self.empty_catch_all.as_deref()
    }
}

/// Why a pattern cannot be expanded. Each variant carries the pattern.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RoutePatternError {
    /// The pattern has more `[` than `]`, or the other way round.
    #[error("the number of `[` and `]` does not match in `{0}`")]
    UnbalancedBrackets(String),

    /// A `]` is followed by more of the route.
    #[error("optional parts can only be at the end of `{0}`")]
    OptionalNotAtEnd(String),

    /// An optional part holds nothing, as in `/a[[/b]]`.
    #[error("`{0}` has an empty optional part")]
    EmptyOptionalPart(String),

    /// A `{` has no matching `}`, or a `}` has no matching `{`.
    #[error("`{0}` has a brace without its partner")]
    UnbalancedBraces(String),

    /// A placeholder has no name, as in `{}`.
    #[error("`{0}` has a placeholder without a name")]
    UnnamedPlaceholder(String),

    /// Two placeholders share a name.
    #[error("`{pattern}` uses the parameter `{name}` twice")]
    DuplicateParameter {
        /// The pattern.
        pattern: String,
        /// The repeated parameter name.
        name: String,
    },

    /// A placeholder has an expression other than `.*`.
    #[error("`{pattern}`: the expression `{regex}` of `{name}` is not supported; only `.*` is")]
    UnsupportedRegex {
        /// The pattern.
        pattern: String,
        /// The parameter name.
        name: String,
        /// The expression that is not supported.
        regex: String,
    },

    /// A `{name:.*}` placeholder is not the last part or does not follow a `/`.
    ///
    /// The pattern `{route:.*}` alone matches every path. Register it as the router's
    /// fallback instead.
    #[error("`{0}`: a `.*` placeholder must be the last part and follow a `/`")]
    MisplacedCatchAll(String),
}

/// Expands a Slim route pattern into the explicit routes it stands for.
///
/// The routes come in order of length: the pattern without its optional parts first.
pub fn expand(pattern: &str) -> Result<Vec<ExpandedRoute>, RoutePatternError> {
    let segments = split_optional_parts(pattern, tokenize(pattern)?)?;

    let mut routes = Vec::new();
    let mut parts = Vec::new();
    for segment in segments {
        parts.extend(segment);
        push_routes(pattern, &parts, &mut routes)?;
    }
    Ok(routes)
}

/// A piece of a route.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    /// Text that must match as it is.
    Text(String),
    /// `{name}`: one path segment.
    Param(String),
    /// `{name:.*}`: the rest of the path.
    CatchAll(String),
}

/// A piece of a pattern: a route part, or a bracket around an optional part.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Part(Part),
    Open,
    Close,
}

/// Splits a pattern into text, placeholders and brackets.
fn tokenize(pattern: &str) -> Result<Vec<Token>, RoutePatternError> {
    fn flush(text: &mut String, tokens: &mut Vec<Token>) {
        if !text.is_empty() {
            tokens.push(Token::Part(Part::Text(std::mem::take(text))));
        }
    }

    let mut tokens = Vec::new();
    let mut text = String::new();
    let mut characters = pattern.chars();
    while let Some(character) = characters.next() {
        match character {
            '{' => {
                flush(&mut text, &mut tokens);
                let body = placeholder_body(&mut characters)
                    .ok_or_else(|| RoutePatternError::UnbalancedBraces(pattern.to_owned()))?;
                tokens.push(Token::Part(placeholder(pattern, &body)?));
            }
            '}' => return Err(RoutePatternError::UnbalancedBraces(pattern.to_owned())),
            '[' => {
                flush(&mut text, &mut tokens);
                tokens.push(Token::Open);
            }
            ']' => {
                flush(&mut text, &mut tokens);
                tokens.push(Token::Close);
            }
            other => text.push(other),
        }
    }
    flush(&mut text, &mut tokens);
    Ok(tokens)
}

/// Reads a placeholder up to its closing brace. The opening brace is already consumed.
///
/// Braces inside the placeholder's expression belong to the expression.
fn placeholder_body(characters: &mut std::str::Chars<'_>) -> Option<String> {
    let mut body = String::new();
    let mut depth = 0_u32;
    for character in characters.by_ref() {
        match character {
            '{' => depth = depth.saturating_add(1),
            '}' if depth == 0 => return Some(body),
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        body.push(character);
    }
    None
}

/// Turns the text between a placeholder's braces into a route part.
fn placeholder(pattern: &str, body: &str) -> Result<Part, RoutePatternError> {
    let (name, regex) = match body.split_once(':') {
        Some((name, regex)) => (name.trim(), Some(regex.trim())),
        None => (body.trim(), None),
    };
    if name.is_empty() {
        return Err(RoutePatternError::UnnamedPlaceholder(pattern.to_owned()));
    }
    match regex {
        None => Ok(Part::Param(name.to_owned())),
        Some(".*") => Ok(Part::CatchAll(name.to_owned())),
        Some(regex) => Err(RoutePatternError::UnsupportedRegex {
            pattern: pattern.to_owned(),
            name: name.to_owned(),
            regex: regex.to_owned(),
        }),
    }
}

/// Splits the tokens at each `[`: the required part first, then one segment per optional part.
fn split_optional_parts(
    pattern: &str,
    mut tokens: Vec<Token>,
) -> Result<Vec<Vec<Part>>, RoutePatternError> {
    let mut closing = 0_usize;
    while tokens.last() == Some(&Token::Close) {
        tokens.pop();
        closing = closing.saturating_add(1);
    }

    let mut segments = vec![Vec::new()];
    for token in tokens {
        match token {
            Token::Part(part) => {
                if let Some(segment) = segments.last_mut() {
                    segment.push(part);
                }
            }
            Token::Open => segments.push(Vec::new()),
            Token::Close => return Err(RoutePatternError::OptionalNotAtEnd(pattern.to_owned())),
        }
    }

    if segments.len().saturating_sub(1) != closing {
        return Err(RoutePatternError::UnbalancedBrackets(pattern.to_owned()));
    }
    if segments.iter().skip(1).any(Vec::is_empty) {
        return Err(RoutePatternError::EmptyOptionalPart(pattern.to_owned()));
    }
    Ok(segments)
}

/// Adds the routes for one level of the pattern: the parts up to an optional part.
fn push_routes(
    pattern: &str,
    parts: &[Part],
    routes: &mut Vec<ExpandedRoute>,
) -> Result<(), RoutePatternError> {
    let mut path = String::new();
    let mut params: Vec<String> = Vec::new();

    let mut remaining = parts.iter().peekable();
    while let Some(part) = remaining.next() {
        match part {
            Part::Text(text) => path.push_str(text),
            Part::Param(name) => {
                ensure_unused(pattern, &params, name)?;
                push_parameter(&mut path, "{p", params.len());
                params.push(name.clone());
            }
            Part::CatchAll(name) => {
                if remaining.peek().is_some() || !path.ends_with('/') {
                    return Err(RoutePatternError::MisplacedCatchAll(pattern.to_owned()));
                }
                ensure_unused(pattern, &params, name)?;
                routes.push(ExpandedRoute {
                    path: path.clone(),
                    params: params.clone(),
                    empty_catch_all: Some(name.clone()),
                });
                push_parameter(&mut path, "{*p", params.len());
                params.push(name.clone());
            }
        }
    }

    routes.push(ExpandedRoute {
        path,
        params,
        empty_catch_all: None,
    });
    Ok(())
}

/// Appends a positional parameter to an Axum path: the opening, the position and `}`.
fn push_parameter(path: &mut String, opening: &str, position: usize) {
    path.push_str(opening);
    path.push_str(&position.to_string());
    path.push('}');
}

/// Fails when `name` is already a parameter of the route.
fn ensure_unused(pattern: &str, params: &[String], name: &str) -> Result<(), RoutePatternError> {
    if params.iter().any(|param| param == name) {
        return Err(RoutePatternError::DuplicateParameter {
            pattern: pattern.to_owned(),
            name: name.to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{RoutePatternError, expand};

    /// The Axum paths a pattern expands to.
    fn paths(pattern: &str) -> Vec<String> {
        expand(pattern)
            .expect("the pattern is valid")
            .iter()
            .map(|route| route.path().to_owned())
            .collect()
    }

    /// The parameter names of each route a pattern expands to.
    fn params(pattern: &str) -> Vec<Vec<String>> {
        expand(pattern)
            .expect("the pattern is valid")
            .iter()
            .map(|route| route.params().to_vec())
            .collect()
    }

    #[test]
    fn static_pattern_is_one_route() {
        assert_eq!(paths("/users"), ["/users"]);
        assert_eq!(
            paths("/plugins/php-mailer/email/test"),
            ["/plugins/php-mailer/email/test"]
        );
    }

    #[test]
    fn empty_pattern_and_slash_are_different_routes() {
        assert_eq!(paths(""), [""]);
        assert_eq!(paths("/"), ["/"]);
    }

    #[test]
    fn placeholder_becomes_a_positional_parameter() {
        assert_eq!(paths("/users/{id}"), ["/users/{p0}"]);
        assert_eq!(params("/users/{id}"), [["id"]]);
        assert_eq!(
            paths("/homepage/overseerr/requests/{type}/{id}/available"),
            ["/homepage/overseerr/requests/{p0}/{p1}/available"]
        );
    }

    #[test]
    fn patterns_of_the_same_shape_share_one_path() {
        assert_eq!(paths("/requests/{type}/{limit}"), ["/requests/{p0}/{p1}"]);
        assert_eq!(paths("/requests/{type}/{id}"), ["/requests/{p0}/{p1}"]);
    }

    #[test]
    fn optional_trailing_slash_gives_both_forms() {
        assert_eq!(paths("/status[/]"), ["/status", "/status/"]);
    }

    #[test]
    fn nested_optional_parts_give_one_route_per_level() {
        assert_eq!(
            paths("/log[/{number}[/{trace_id}]]"),
            ["/log", "/log/{p0}", "/log/{p0}/{p1}"]
        );
        assert_eq!(
            params("/log[/{number}[/{trace_id}]]"),
            [vec![], vec!["number"], vec!["number", "trace_id"]]
        );
    }

    #[test]
    fn forward_auth_pattern_gives_five_routes() {
        assert_eq!(
            paths("/auth[/[{group}[/{type}[/{ips}]]]]"),
            [
                "/auth",
                "/auth/",
                "/auth/{p0}",
                "/auth/{p0}/{p1}",
                "/auth/{p0}/{p1}/{p2}"
            ]
        );
    }

    #[test]
    fn placeholder_can_follow_text_inside_a_segment() {
        assert_eq!(
            paths("/auth-[{group}[/]]"),
            ["/auth-", "/auth-{p0}", "/auth-{p0}/"]
        );
    }

    #[test]
    fn optional_placeholder_after_a_slash_keeps_the_slash() {
        assert_eq!(
            paths("/homepage/donate/[{amount}]"),
            ["/homepage/donate/", "/homepage/donate/{p0}"]
        );
    }

    #[test]
    fn catch_all_gives_a_route_for_the_empty_rest_and_a_wildcard_route() {
        let routes = expand("/socks/{app}/{route:.*}").expect("the pattern is valid");

        let [empty, wildcard] = routes.as_slice() else {
            panic!("expected two routes, got {routes:?}");
        };
        assert_eq!(empty.path(), "/socks/{p0}/");
        assert_eq!(empty.params(), ["app"]);
        assert_eq!(empty.empty_catch_all(), Some("route"));
        assert_eq!(wildcard.path(), "/socks/{p0}/{*p1}");
        assert_eq!(wildcard.params(), ["app", "route"]);
        assert_eq!(wildcard.empty_catch_all(), None);
    }

    #[test]
    fn unbalanced_brackets_are_rejected() {
        assert_eq!(
            expand("/a[/b"),
            Err(RoutePatternError::UnbalancedBrackets("/a[/b".to_owned()))
        );
        assert_eq!(
            expand("/a/b]"),
            Err(RoutePatternError::UnbalancedBrackets("/a/b]".to_owned()))
        );
    }

    #[test]
    fn optional_part_in_the_middle_is_rejected() {
        assert_eq!(
            expand("/a[/b]/c"),
            Err(RoutePatternError::OptionalNotAtEnd("/a[/b]/c".to_owned()))
        );
    }

    #[test]
    fn empty_optional_part_is_rejected() {
        assert_eq!(
            expand("/a[[/b]]"),
            Err(RoutePatternError::EmptyOptionalPart("/a[[/b]]".to_owned()))
        );
    }

    #[test]
    fn broken_placeholders_are_rejected() {
        assert_eq!(
            expand("/a/{id"),
            Err(RoutePatternError::UnbalancedBraces("/a/{id".to_owned()))
        );
        assert_eq!(
            expand("/a/id}"),
            Err(RoutePatternError::UnbalancedBraces("/a/id}".to_owned()))
        );
        assert_eq!(
            expand("/a/{}"),
            Err(RoutePatternError::UnnamedPlaceholder("/a/{}".to_owned()))
        );
        assert_eq!(
            expand("/a/{x}/{x}"),
            Err(RoutePatternError::DuplicateParameter {
                pattern: "/a/{x}/{x}".to_owned(),
                name: "x".to_owned(),
            })
        );
    }

    #[test]
    fn other_expressions_than_catch_all_are_rejected() {
        assert_eq!(
            expand("/a/{id:[0-9]{1,3}}"),
            Err(RoutePatternError::UnsupportedRegex {
                pattern: "/a/{id:[0-9]{1,3}}".to_owned(),
                name: "id".to_owned(),
                regex: "[0-9]{1,3}".to_owned(),
            })
        );
    }

    #[test]
    fn catch_all_must_be_last_and_follow_a_slash() {
        assert_eq!(
            expand("{route:.*}"),
            Err(RoutePatternError::MisplacedCatchAll(
                "{route:.*}".to_owned()
            ))
        );
        assert_eq!(
            expand("/a/{rest:.*}/b"),
            Err(RoutePatternError::MisplacedCatchAll(
                "/a/{rest:.*}/b".to_owned()
            ))
        );
        assert_eq!(
            expand("/a-{rest:.*}"),
            Err(RoutePatternError::MisplacedCatchAll(
                "/a-{rest:.*}".to_owned()
            ))
        );
    }
}
