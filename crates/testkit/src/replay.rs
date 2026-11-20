//! Replays recorded answers of the PHP application against the Rust application.
//!
//! `parity/recorder/record.py` writes the cases to `parity/cases/<group>.json`. A replay
//! sends each recorded request to the router and compares four aspects of the answer:
//! the status code, the recorded headers, the cookies set, and the body, byte for byte.
//!
//! Of a cookie, the name, the domain, the path and the lifetime are compared: `deleted`,
//! `session`, or the seconds until the expiry, rounded to a whole minute. The value is
//! not compared; it is random.
//!
//! A case can name a cookie jar. Its request carries the cookies of the jar, and the
//! cookies of the answer go into the jar, as in a browser. The replay fills its own jars
//! from the answers of the Rust application, so a later case uses the session of an
//! earlier login of the same replay. In a path or a body, `{session_token:<jar>}` stands
//! for the session token in a jar, and the cookie name `{session_cookie}` stands for the
//! name of the session cookie.
//!
//! The recorder replaces values that change with every recording (addresses, tokens,
//! times) by placeholders. The replay replaces the same values in the answers of the Rust
//! application before it compares. What those values look like is tested elsewhere.
//!
//! An aspect may differ from the recording in two ways only:
//!
//! - an **expectation edit**: the expected body is changed, or an expected cookie is
//!   removed, before the comparison. An edit names an approved entry of
//!   `docs/deviations.md`, or a difference of the environment the test runs in.
//! - a **pending marker**: the aspect, the whole case, or one part of a JSON body is not
//!   implemented yet. What is pending must differ. When it starts to match, the test
//!   fails until the marker is removed, so a marker cannot outlive its reason.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, header};
use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::Value;
use tower::ServiceExt;

/// The address that the requests of a replay come from: the gateway of a container
/// network, as for the recorder. The port is of no interest.
pub const RECORDER_ADDRESS: ([u8; 4], u16) = ([172, 18, 0, 1], 40_000);

/// A marker that stands for every case of the group.
pub const EVERY_CASE: &str = "*";

/// One part of an answer that a replay compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
    /// The status code.
    Status,
    /// The headers the recorder keeps, for example `Content-Type` and `Location`.
    Headers,
    /// The cookies the answer sets: name, domain, path and lifetime of each.
    Cookies,
    /// The body, byte for byte.
    Body,
}

const ASPECTS: [Aspect; 4] = [
    Aspect::Status,
    Aspect::Headers,
    Aspect::Cookies,
    Aspect::Body,
];

/// A replay of one group of recorded cases.
#[derive(Debug)]
pub struct Replay {
    group: String,
    edits: Vec<Edit>,
    null_edits: Vec<NullEdit>,
    /// Keys whose values the answer has as `null`: case and keys.
    hidden: Vec<(String, Vec<String>)>,
    /// Keys that the answer does not have: case and JSON pointer.
    removals: Vec<(String, String)>,
    /// Objects whose keys the answer has in sorted order: case and JSON pointer.
    sorted_objects: Vec<(String, String)>,
    pending_aspects: Vec<(String, Aspect)>,
    pending_cases: Vec<String>,
    /// Parts of a JSON body that are not implemented yet: case and JSON pointer.
    pending_parts: Vec<(String, String)>,
    absent_cookies: Vec<String>,
}

#[derive(Debug)]
struct Edit {
    case: String,
    pointer: String,
    /// The key to add to the object at `pointer`. Without it, the value at `pointer`
    /// is replaced.
    new_key: Option<String>,
    /// Whether the keys of the object are put in the order of the alphabet after the
    /// new key is added.
    sorted: bool,
    value: Value,
}

/// A value that the answer has wherever the recorded body of a case has `null` under
/// one of the keys.
#[derive(Debug)]
struct NullEdit {
    case: String,
    keys: Vec<String>,
    value: Value,
}

impl Replay {
    /// Prepares the replay of `parity/cases/<group>.json`.
    pub fn group(group: &str) -> Self {
        Self {
            group: group.to_owned(),
            edits: Vec::new(),
            null_edits: Vec::new(),
            hidden: Vec::new(),
            removals: Vec::new(),
            sorted_objects: Vec::new(),
            pending_aspects: Vec::new(),
            pending_cases: Vec::new(),
            pending_parts: Vec::new(),
            absent_cookies: Vec::new(),
        }
    }

    /// Expects another value in the body of a case, because of an approved deviation.
    ///
    /// `id` must be an entry of `docs/deviations.md`. `pointer` is a JSON pointer into the
    /// recorded body, for example `/response/data/php`.
    #[must_use]
    pub fn deviation(self, id: &str, case: &str, pointer: &str, value: Value) -> Self {
        assert_approved(id);
        self.edit(case, pointer, value)
    }

    /// Expects one more key in an object of the body of a case, because of an approved
    /// deviation. The key comes after the keys the object has.
    ///
    /// `id` must be an entry of `docs/deviations.md`. `pointer` is a JSON pointer to the
    /// object, for example `/response/data/tokenList/0`.
    #[must_use]
    pub fn deviation_adds(
        mut self,
        id: &str,
        case: &str,
        pointer: &str,
        key: &str,
        value: Value,
    ) -> Self {
        assert_approved(id);
        self.edits.push(Edit {
            case: case.to_owned(),
            pointer: pointer.to_owned(),
            new_key: Some(key.to_owned()),
            sorted: false,
            value,
        });
        self
    }

    /// Expects one more key in an object of the body of a case whose keys stand in the
    /// order of the alphabet, because of an approved deviation. The key stands at its
    /// place in that order.
    ///
    /// `id` must be an entry of `docs/deviations.md`. `pointer` is a JSON pointer to the
    /// object, for example `/response/data`.
    #[must_use]
    pub fn deviation_adds_in_order(
        mut self,
        id: &str,
        case: &str,
        pointer: &str,
        key: &str,
        value: Value,
    ) -> Self {
        assert_approved(id);
        self.edits.push(Edit {
            case: case.to_owned(),
            pointer: pointer.to_owned(),
            new_key: Some(key.to_owned()),
            sorted: true,
            value,
        });
        self
    }

    /// Expects that the body of a case does not have a key that the recording has,
    /// because of an approved deviation.
    ///
    /// `id` must be an entry of `docs/deviations.md`. `pointer` is a JSON pointer to a
    /// key of an object, for example `/response/data/status/userConfigPath`.
    #[must_use]
    pub fn deviation_without(mut self, id: &str, case: &str, pointer: &str) -> Self {
        assert_approved(id);
        self.removals.push((case.to_owned(), pointer.to_owned()));
        self
    }

    /// Expects `value` wherever the recorded body of a case has `null` under one of
    /// `keys`, at any depth, because of an approved deviation. Use it for a difference
    /// that comes back in every record of a list.
    ///
    /// `id` must be an entry of `docs/deviations.md`. The recorded body must have at
    /// least one such `null`.
    #[must_use]
    pub fn deviation_for_nulls(
        mut self,
        id: &str,
        case: &str,
        keys: &[&str],
        value: Value,
    ) -> Self {
        assert_approved(id);
        self.null_edits.push(NullEdit {
            case: case.to_owned(),
            keys: keys.iter().map(|key| (*key).to_owned()).collect(),
            value,
        });
        self
    }

    /// Expects `null` wherever the recorded body of a case has a value under one of
    /// `keys`, at any depth, because of an approved deviation. This is for a value that
    /// the application never shows, in every record of a list.
    ///
    /// `id` must be an entry of `docs/deviations.md`. The recorded body must have at
    /// least one such value.
    #[must_use]
    pub fn deviation_hides(mut self, id: &str, case: &str, keys: &[&str]) -> Self {
        assert_approved(id);
        self.hidden.push((
            case.to_owned(),
            keys.iter().map(|key| (*key).to_owned()).collect(),
        ));
        self
    }

    /// Expects that no case sets the named cookie, because of an approved deviation.
    ///
    /// `id` must be an entry of `docs/deviations.md`.
    #[must_use]
    pub fn without_cookie(mut self, id: &str, name: &str) -> Self {
        assert_approved(id);
        self.absent_cookies.push(name.to_owned());
        self
    }

    /// Expects another value in the body of a case, because the test does not run where
    /// the recording was made. `reason` says what differs, for the reader of the test.
    #[must_use]
    pub fn environment(self, reason: &str, case: &str, pointer: &str, value: Value) -> Self {
        assert!(!reason.is_empty(), "an environment edit needs a reason");
        self.edit(case, pointer, value)
    }

    /// Marks one aspect of a case, or of [`EVERY_CASE`], as not implemented yet.
    #[must_use]
    pub fn pending(mut self, case: &str, aspect: Aspect, reason: &str) -> Self {
        assert!(!reason.is_empty(), "a pending marker needs a reason");
        self.pending_aspects.push((case.to_owned(), aspect));
        self
    }

    /// Marks one part of the JSON body of a case as not implemented yet. `pointer` is a
    /// JSON pointer to a key of an object, for example `/response/data/settings`.
    ///
    /// The part is taken out of the recorded body and out of the answer before the two
    /// are compared, so the rest of the body is still held to the recording. The part
    /// itself must differ: the answer has another value there, or no such key.
    ///
    /// Both bodies are written again for the comparison, so for such a case the replay
    /// compares the JSON values, not every byte.
    #[must_use]
    pub fn pending_at(mut self, case: &str, pointer: &str, reason: &str) -> Self {
        assert!(!reason.is_empty(), "a pending marker needs a reason");
        self.pending_parts
            .push((case.to_owned(), pointer.to_owned()));
        self
    }

    /// Marks a whole case as not implemented yet.
    #[must_use]
    pub fn pending_case(mut self, case: &str, reason: &str) -> Self {
        assert!(!reason.is_empty(), "a pending marker needs a reason");
        self.pending_cases.push(case.to_owned());
        self
    }

    fn edit(mut self, case: &str, pointer: &str, value: Value) -> Self {
        self.edits.push(Edit {
            case: case.to_owned(),
            pointer: pointer.to_owned(),
            new_key: None,
            sorted: false,
            value,
        });
        self
    }

    /// Sends every recorded request to `app` and panics with a report when an answer
    /// differs from the recording in a way that is not allowed.
    pub async fn run(&self, app: &Router) {
        let recording = Recording::load(&self.group);
        self.check_names(&recording);

        let mut problems = String::new();
        let mut browsers = Browsers::default();
        for case in &recording.cases {
            let mut expected = self.expected(case);
            let mut actual = Answer::of(app, &recording, case, &mut browsers).await;
            self.take_out_pending_parts(case, &mut expected, &mut actual, &mut problems);
            let differing: Vec<Aspect> = ASPECTS
                .into_iter()
                .filter(|aspect| !expected.matches(&actual, *aspect))
                .collect();

            if self.pending_cases.contains(&case.name) {
                let implemented = ASPECTS
                    .into_iter()
                    .filter(|aspect| !self.is_pending(&case.name, *aspect))
                    .all(|aspect| !differing.contains(&aspect));
                if implemented {
                    let _ = writeln!(
                        problems,
                        "case `{}` is marked pending but now matches; remove the marker",
                        case.name
                    );
                }
                continue;
            }

            for aspect in ASPECTS {
                let differs = differing.contains(&aspect);
                if self.is_pending(&case.name, aspect) {
                    if !differs {
                        let _ = writeln!(
                            problems,
                            "case `{}`: {aspect:?} is marked pending but now matches; \
                             remove the marker",
                            case.name
                        );
                    }
                } else if differs {
                    let (php, rust) =
                        at_first_difference(&expected.show(aspect), &actual.show(aspect));
                    let _ = writeln!(
                        problems,
                        "case `{}`: {aspect:?} differs\n  PHP:  {php}\n  Rust: {rust}",
                        case.name,
                    );
                }
            }
        }

        assert!(
            problems.is_empty(),
            "replay of `{}` against PHP {}:\n{problems}",
            self.group,
            recording.source.php_version
        );
    }

    fn is_pending(&self, case: &str, aspect: Aspect) -> bool {
        self.pending_aspects
            .iter()
            .any(|(name, pending)| *pending == aspect && (name == case || name == EVERY_CASE))
    }

    /// Expects the keys of an object in sorted order, because of an approved deviation.
    ///
    /// The values stay as recorded. The recording must have the keys in another order.
    #[must_use]
    pub fn deviation_sorts_keys(mut self, id: &str, case: &str, pointer: &str) -> Self {
        assert_approved(id);
        self.sorted_objects
            .push((case.to_owned(), pointer.to_owned()));
        self
    }

    /// The recorded answer of a case, with the expectation edits applied.
    fn expected(&self, case: &Case) -> Answer {
        let mut body = case.response.body.clone();
        let edits: Vec<&Edit> = self
            .edits
            .iter()
            .filter(|edit| edit.case == case.name)
            .collect();
        let null_edits: Vec<&NullEdit> = self
            .null_edits
            .iter()
            .filter(|edit| edit.case == case.name)
            .collect();
        let removals: Vec<&str> = self
            .removals
            .iter()
            .filter(|(name, _)| name == &case.name)
            .map(|(_, pointer)| pointer.as_str())
            .collect();
        let sorted_objects: Vec<&str> = self
            .sorted_objects
            .iter()
            .filter(|(name, _)| name == &case.name)
            .map(|(_, pointer)| pointer.as_str())
            .collect();
        let hidden: Vec<&[String]> = self
            .hidden
            .iter()
            .filter(|(name, _)| name == &case.name)
            .map(|(_, keys)| keys.as_slice())
            .collect();
        if !edits.is_empty()
            || !null_edits.is_empty()
            || !hidden.is_empty()
            || !removals.is_empty()
            || !sorted_objects.is_empty()
        {
            let mut json: Value =
                serde_json::from_str(&body).expect("a case with edits has a JSON body");
            for edit in null_edits {
                let replaced = replace_nulls(&mut json, &edit.keys, &edit.value);
                assert!(
                    replaced > 0,
                    "case `{}`: the recorded body has no `null` under {:?}",
                    case.name,
                    edit.keys
                );
            }
            for keys in hidden {
                assert!(
                    hide_values(&mut json, keys) > 0,
                    "case `{}`: the recorded body has no value under {keys:?}",
                    case.name
                );
            }
            for edit in edits {
                edit.apply(&mut json, &case.name);
            }
            // Last, so that a key can be taken out that an edit above has put in.
            for pointer in removals {
                assert!(
                    take_part(&mut json, pointer).is_some(),
                    "case `{}`: the recorded body has nothing at `{pointer}`",
                    case.name
                );
            }
            for pointer in sorted_objects {
                sort_keys_at(&mut json, pointer, &case.name);
            }
            let bytes = organizr_api::php_json::to_vec(&json).expect("the body can be written");
            body = String::from_utf8(bytes).expect("the writer produces UTF-8");
        }

        let mut cookies: Vec<String> = case
            .response
            .cookies_set
            .iter()
            .filter(|cookie| !self.absent_cookies.contains(&cookie.name))
            .map(RecordedCookie::describe)
            .collect();
        cookies.sort();
        Answer {
            status: case.response.status,
            headers: case.response.headers.clone(),
            cookies,
            body,
        }
    }

    /// Takes the pending parts of a case out of the recorded body and out of the answer.
    /// Reports a part that the answer already has as the recording has it.
    fn take_out_pending_parts(
        &self,
        case: &Case,
        expected: &mut Answer,
        actual: &mut Answer,
        problems: &mut String,
    ) {
        let pointers: Vec<&str> = self
            .pending_parts
            .iter()
            .filter(|(name, _)| name == &case.name)
            .map(|(_, pointer)| pointer.as_str())
            .collect();
        if pointers.is_empty() {
            return;
        }
        let mut recorded: Value = serde_json::from_str(&expected.body)
            .expect("a case with pending parts has a JSON body");
        // An answer that is not JSON differs as a whole. Leave it as it is.
        let Ok(mut answered) = serde_json::from_str::<Value>(&actual.body) else {
            return;
        };

        for pointer in pointers {
            let recorded_part = take_part(&mut recorded, pointer).unwrap_or_else(|| {
                panic!(
                    "case `{}`: the recorded body has nothing at `{pointer}`",
                    case.name
                )
            });
            if take_part(&mut answered, pointer) == Some(recorded_part) {
                let _ = writeln!(
                    problems,
                    "case `{}`: the part at `{pointer}` is marked pending but now matches; \
                     remove the marker",
                    case.name
                );
            }
        }

        let write = |json: &Value| {
            let bytes = organizr_api::php_json::to_vec(json).expect("the body can be written");
            String::from_utf8(bytes).expect("the writer produces UTF-8")
        };
        expected.body = write(&recorded);
        actual.body = write(&answered);
    }

    /// Fails when an edit or a marker names a case the group does not have.
    fn check_names(&self, recording: &Recording) {
        let named = self
            .edits
            .iter()
            .map(|edit| &edit.case)
            .chain(self.null_edits.iter().map(|edit| &edit.case))
            .chain(self.removals.iter().map(|(case, _)| case))
            .chain(self.pending_aspects.iter().map(|(case, _)| case))
            .chain(self.pending_parts.iter().map(|(case, _)| case))
            .chain(&self.pending_cases);
        for name in named {
            assert!(
                name == EVERY_CASE || recording.cases.iter().any(|case| &case.name == name),
                "the group `{}` has no case `{name}`",
                self.group
            );
        }
    }
}

/// Replaces every `null` that stands under one of `keys`, at any depth, by `value`.
/// Returns how many were replaced.
fn replace_nulls(json: &mut Value, keys: &[String], value: &Value) -> usize {
    match json {
        Value::Object(object) => object
            .iter_mut()
            .map(|(key, member)| {
                if member.is_null() && keys.contains(key) {
                    *member = value.clone();
                    1
                } else {
                    replace_nulls(member, keys, value)
                }
            })
            .sum(),
        Value::Array(entries) => entries
            .iter_mut()
            .map(|entry| replace_nulls(entry, keys, value))
            .sum(),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => 0,
    }
}

/// Replaces every value but `null` that stands under one of `keys`, at any depth, by
/// `null`. Returns how many were replaced.
fn hide_values(json: &mut Value, keys: &[String]) -> usize {
    match json {
        Value::Object(object) => object
            .iter_mut()
            .map(|(key, member)| {
                if !member.is_null() && keys.contains(key) {
                    *member = Value::Null;
                    1
                } else {
                    hide_values(member, keys)
                }
            })
            .sum(),
        Value::Array(entries) => entries
            .iter_mut()
            .map(|entry| hide_values(entry, keys))
            .sum(),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => 0,
    }
}

/// Removes the value at a JSON pointer to a key of an object, and returns it.
fn take_part(json: &mut Value, pointer: &str) -> Option<Value> {
    let (parent, key) = pointer.rsplit_once('/')?;
    json.pointer_mut(parent)?.as_object_mut()?.shift_remove(key)
}

/// Fails unless `id` is an entry of `docs/deviations.md`.
fn assert_approved(id: &str) {
    let approved = std::fs::read_to_string(repository_root().join("docs/deviations.md"))
        .expect("docs/deviations.md can be read");
    assert!(
        approved.contains(&format!("| {id} |")),
        "{id} is not an entry of docs/deviations.md; a deviation needs approval first"
    );
}

/// The compared parts of an answer, recorded or actual.
#[derive(Debug)]
struct Answer {
    status: u16,
    /// The recorded headers that are present, by lower-case name.
    headers: BTreeMap<String, String>,
    /// The cookies set, each as [`RecordedCookie::describe`] writes it, sorted.
    cookies: Vec<String>,
    body: String,
}

/// What a replay remembers from case to case.
#[derive(Debug, Default)]
struct Browsers {
    /// The cookie jars, by name: cookie name to value.
    jars: HashMap<String, BTreeMap<String, String>>,
    /// The name of the session cookie, once an answer has set the cookie.
    session_cookie: Option<String>,
    /// The secret of the second factor that the application made last.
    second_factor_secret: Option<String>,
}

impl Browsers {
    /// Stands for the name of the session cookie in the `cookies` of a case.
    const SESSION_COOKIE: &'static str = "{session_cookie}";

    /// Starts the placeholder for the session token of a jar.
    const SESSION_TOKEN: &'static str = "{session_token:";

    fn session_cookie(&self) -> &str {
        self.session_cookie
            .as_deref()
            .expect("an earlier case has set the session cookie")
    }

    /// Replaces each placeholder of the second factor by its value: `{2fa_secret}` is
    /// the secret that the application made last, `{2fa_code}` its code of this moment,
    /// `{2fa_code:-5}` the code of five steps of 30 seconds ago, and `{2fa_wrong_code}`
    /// six digits that are no code of the steps around this moment.
    fn fill_in_second_factor(&self, text: &str) -> String {
        const START: &str = "{2fa_";
        let mut filled = String::new();
        let mut rest = text;
        while let Some((before, after)) = rest.split_once(START) {
            let (placeholder, after) = after
                .split_once('}')
                .expect("the placeholder ends with a brace");
            let secret = self
                .second_factor_secret
                .as_deref()
                .expect("an earlier answer held the secret of a second factor");
            let now = jiff::Timestamp::now().as_second();
            let code_of_step = |steps: i64| {
                let time = u64::try_from(now + steps * 30).expect("the time is after 1970");
                organizr_auth::two_factor::code_at(secret, time).expect("the secret is valid")
            };
            let value = match placeholder.split_once(':') {
                None if placeholder == "secret" => secret.to_owned(),
                None if placeholder == "code" => code_of_step(0),
                None if placeholder == "wrong_code" => {
                    let near: Vec<String> = (-8..=8).map(code_of_step).collect();
                    (0..1_000_000)
                        .map(|number| format!("{number:06}"))
                        .find(|code| !near.contains(code))
                        .expect("some code is wrong")
                }
                Some(("code", steps)) => {
                    code_of_step(steps.parse().expect("the steps are a whole number"))
                }
                _ => panic!("`{{2fa_{placeholder}}}` is no placeholder of the second factor"),
            };
            filled.push_str(before);
            filled.push_str(&value);
            rest = after;
        }
        filled.push_str(rest);
        filled
    }

    /// Replaces each `{session_token:<jar>}` by the session token of that jar, and each
    /// placeholder of the second factor by its value.
    fn fill_in(&self, text: &str) -> String {
        // `{client_ip}` is the address that the reference saw the recorder at.
        let text = self
            .fill_in_second_factor(text)
            .replace("{client_ip}", &IpAddr::from(RECORDER_ADDRESS.0).to_string());
        let mut filled = String::new();
        let mut rest = text.as_str();
        while let Some((before, after)) = rest.split_once(Self::SESSION_TOKEN) {
            let (jar, after) = after
                .split_once('}')
                .expect("the placeholder ends with a brace");
            let token = self
                .jars
                .get(jar)
                .and_then(|jar| jar.get(self.session_cookie()))
                .unwrap_or_else(|| panic!("the jar `{jar}` holds no session token"));
            filled.push_str(before);
            filled.push_str(token);
            rest = after;
        }
        filled.push_str(rest);
        filled
    }
}

/// What stands for a picture that is inside an answer as a `data:` address.
pub const PICTURE: &str = "<picture>";

/// Replaces each PNG picture that is inside a body as a `data:` address by [`PICTURE`].
/// Such a picture is made from a random value, so no two answers hold the same one.
fn without_pictures(body: &str) -> String {
    const START: &str = "\"data:image/png;base64,";
    let mut cleaned = String::with_capacity(body.len());
    let mut rest = body;
    while let Some((before, after)) = rest.split_once(START) {
        let (_, after) = after.split_once('"').expect("the text ends with a quote");
        cleaned.push_str(before);
        cleaned.push('"');
        cleaned.push_str(PICTURE);
        cleaned.push('"');
        rest = after;
    }
    cleaned.push_str(rest);
    cleaned
}

/// Replaces the text value of every `key` in a JSON body by `placeholder`, as the
/// recorder does. A value that is empty, or not text, stays as it is.
fn redact(body: &str, key: &str, placeholder: &str) -> String {
    let marker = format!("\"{key}\": \"");
    let mut redacted = String::with_capacity(body.len());
    let mut rest = body;
    while let Some((before, after)) = rest.split_once(&marker) {
        redacted.push_str(before);
        redacted.push_str(&marker);
        // The value ends at the next quote. A value with an escape is left alone.
        match after.split_once(['"', '\\']) {
            Some((value, _)) if !value.is_empty() && after[value.len()..].starts_with('"') => {
                redacted.push_str(placeholder);
                rest = &after[value.len()..];
            }
            _ => rest = after,
        }
    }
    redacted.push_str(rest);
    redacted
}

impl Answer {
    /// Sends the request of a case to the application and takes the answer apart.
    async fn of(app: &Router, recording: &Recording, case: &Case, browsers: &mut Browsers) -> Self {
        let mut request = Request::builder()
            .method(case.request.method.as_str())
            .uri(browsers.fill_in(&case.request.path))
            .header(header::HOST, &recording.source.host);
        let body = match (&case.request.json, &case.request.body) {
            (Some(json), _) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                let text = serde_json::to_string(json).expect("the request body can be written");
                Body::from(browsers.fill_in(&text))
            }
            (None, Some(text)) => Body::from(browsers.fill_in(text)),
            (None, None) => Body::empty(),
        };
        for (name, value) in &case.request.headers {
            request = request.header(name.as_str(), browsers.fill_in(value));
        }
        let mut sent_cookies = BTreeMap::new();
        for (name, value) in &case.request.cookies {
            let name = if name == Browsers::SESSION_COOKIE {
                browsers.session_cookie()
            } else {
                name
            };
            sent_cookies.insert(name.to_owned(), value.clone());
        }
        if let Some(jar) = &case.request.jar {
            sent_cookies.extend(browsers.jars.entry(jar.clone()).or_default().clone());
        }
        if !sent_cookies.is_empty() {
            let cookies: Vec<String> = sent_cookies
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            request = request.header(header::COOKIE, cookies.join("; "));
        }
        let mut request = request.body(body).expect("the recorded request is valid");
        // The recorder reaches the reference through the gateway of the container
        // network, which is a private address. So the reference takes the recorder for
        // a caller on the local network, and the replay must look like one too.
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(RECORDER_ADDRESS)));
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("the router answers");

        let status = response.status().as_u16();
        let mut headers = BTreeMap::new();
        for name in &recording.source.recorded_headers {
            if let Some(value) = response.headers().get(name) {
                let value = value.to_str().expect("a header value is text");
                headers.insert(name.clone(), value.to_owned());
            }
        }
        let now = jiff::Timestamp::now();
        let mut cookies = Vec::new();
        for value in response.headers().get_all(header::SET_COOKIE) {
            let value = value.to_str().expect("a cookie is text");
            let (name, cookie_value, cookie) = RecordedCookie::parse(value, now);
            if cookie.name == RecordedCookie::SESSION_COOKIE {
                browsers.session_cookie = Some(name.clone());
            }
            if let Some(jar) = &case.request.jar {
                let jar = browsers.jars.entry(jar.clone()).or_default();
                if cookie.lifetime == "deleted" {
                    jar.remove(&name);
                } else {
                    jar.insert(name, cookie_value);
                }
            }
            cookies.push(cookie.describe());
        }
        cookies.sort();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("the body can be read")
            .to_bytes();

        let mut body = String::from_utf8(bytes.to_vec()).expect("the body is UTF-8");
        // An answer that holds the secret of a second factor: later cases use the secret.
        let made = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|json| {
                json.pointer("/response/data/secret")?
                    .as_str()
                    .map(str::to_owned)
            });
        if let Some(secret) = made.filter(|secret| !secret.is_empty()) {
            browsers.second_factor_secret = Some(secret);
        }
        body = without_pictures(&body);
        for (key, placeholder) in &recording.source.redacted_keys {
            body = redact(&body, key, placeholder);
        }

        Self {
            status,
            headers,
            cookies,
            body,
        }
    }

    fn matches(&self, other: &Self, aspect: Aspect) -> bool {
        match aspect {
            Aspect::Status => self.status == other.status,
            Aspect::Headers => self.headers == other.headers,
            Aspect::Cookies => self.cookies == other.cookies,
            Aspect::Body => self.body == other.body,
        }
    }

    fn show(&self, aspect: Aspect) -> String {
        match aspect {
            Aspect::Status => self.status.to_string(),
            Aspect::Headers => format!("{:?}", self.headers),
            Aspect::Cookies => format!("{:?}", self.cookies),
            Aspect::Body => format!("{:?}", self.body),
        }
    }
}

/// A file of recorded cases.
#[derive(Debug, Deserialize)]
struct Recording {
    source: Source,
    cases: Vec<Case>,
}

impl Recording {
    fn load(group: &str) -> Self {
        let path = repository_root()
            .join("parity/cases")
            .join(format!("{group}.json"));
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} cannot be read: {error}", path.display()));
        serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("{} is not valid: {error}", path.display()))
    }
}

#[derive(Debug, Deserialize)]
struct Source {
    php_version: String,
    host: String,
    recorded_headers: Vec<String>,
    /// Keys of a JSON body whose text value the recorder replaced, with the placeholder.
    #[serde(default)]
    redacted_keys: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    request: RecordedRequest,
    response: RecordedResponse,
}

#[derive(Debug, Deserialize)]
struct RecordedRequest {
    method: String,
    path: String,
    /// A value to send as a JSON body.
    json: Option<Value>,
    /// A text to send as the body, as it is.
    body: Option<String>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    /// Cookies to send with fixed values.
    #[serde(default)]
    cookies: BTreeMap<String, String>,
    /// The cookie jar the request uses.
    jar: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RecordedResponse {
    status: u16,
    headers: BTreeMap<String, String>,
    cookies_set: Vec<RecordedCookie>,
    body: String,
}

/// What is compared of a cookie that an answer sets.
#[derive(Debug, Deserialize)]
struct RecordedCookie {
    /// The name. The session cookie is named `organizr_token_<uuid>`, whatever the id of
    /// the install is.
    name: String,
    domain: Option<String>,
    path: Option<String>,
    /// `deleted`, `session`, or seconds rounded to a whole minute, for example `86400s`.
    lifetime: String,
}

impl RecordedCookie {
    /// The name that stands for the session cookie of any install.
    const SESSION_COOKIE: &'static str = "organizr_token_<uuid>";

    /// How `parity/recorder/record.py` and the Rust application write an expiry.
    const EXPIRY_FORMAT: &'static str = "%a, %d-%b-%Y %H:%M:%S GMT";

    /// Takes a `Set-Cookie` value apart, as the recorder does: the name as sent, the
    /// value, and what is compared.
    fn parse(header: &str, now: jiff::Timestamp) -> (String, String, Self) {
        let mut parts = header.split(';').map(str::trim);
        let (name, value) = parts
            .next()
            .and_then(|first| first.split_once('='))
            .unwrap_or_else(|| panic!("`{header}` is not a cookie"));
        let attributes: BTreeMap<String, &str> = parts
            .map(|part| {
                let (key, attribute) = part.split_once('=').unwrap_or((part, ""));
                (key.to_lowercase(), attribute)
            })
            .collect();

        let lifetime = match attributes.get("expires") {
            None => "session".to_owned(),
            Some(expires) => {
                let expires = jiff::civil::DateTime::strptime(Self::EXPIRY_FORMAT, expires)
                    .and_then(|time| jiff::tz::TimeZone::UTC.to_timestamp(time))
                    .unwrap_or_else(|error| panic!("the expiry of `{header}`: {error}"));
                let seconds = expires.as_second() - now.as_second();
                if seconds < 0 {
                    "deleted".to_owned()
                } else {
                    format!("{}s", (seconds + 30) / 60 * 60)
                }
            }
        };

        let install_id = name.strip_prefix("organizr_token_").filter(|id| {
            id.len() == 36
                && id
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() || byte == b'-')
        });
        let cookie = Self {
            name: if install_id.is_some() {
                Self::SESSION_COOKIE.to_owned()
            } else {
                name.to_owned()
            },
            domain: attributes.get("domain").map(|domain| (*domain).to_owned()),
            path: attributes.get("path").map(|path| (*path).to_owned()),
            lifetime,
        };
        (name.to_owned(), value.to_owned(), cookie)
    }

    /// One line that shows everything that is compared.
    fn describe(&self) -> String {
        format!(
            "{}; domain={}; path={}; lifetime={}",
            self.name,
            self.domain.as_deref().unwrap_or("none"),
            self.path.as_deref().unwrap_or("none"),
            self.lifetime
        )
    }
}

/// The root of the repository, two levels above this crate.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

impl Edit {
    /// Makes the change in the recorded body of the case.
    fn apply(&self, json: &mut Value, case: &str) {
        let target = json.pointer_mut(&self.pointer).unwrap_or_else(|| {
            panic!(
                "case `{case}`: the recorded body has nothing at `{}`",
                self.pointer
            )
        });
        match &self.new_key {
            None => *target = self.value.clone(),
            Some(key) => {
                let object = target.as_object_mut().unwrap_or_else(|| {
                    panic!(
                        "case `{case}`: the recorded body has no object at `{}`",
                        self.pointer
                    )
                });
                let replaced = object.insert(key.clone(), self.value.clone());
                assert!(
                    replaced.is_none(),
                    "case `{case}`: the object at `{}` already has the key `{key}`",
                    self.pointer
                );
                if self.sorted {
                    object.sort_keys();
                }
            }
        }
    }
}

/// Sorts the keys of the object at `pointer` in the recorded body of the case. The keys
/// must be in another order before.
fn sort_keys_at(json: &mut Value, pointer: &str, case: &str) {
    let object = json
        .pointer_mut(pointer)
        .and_then(Value::as_object_mut)
        .unwrap_or_else(|| panic!("case `{case}`: the recorded body has no object at `{pointer}`"));
    let recorded: Vec<String> = object.keys().cloned().collect();
    object.sort_keys();
    assert!(
        object.keys().ne(recorded.iter()),
        "case `{case}`: the recorded keys at `{pointer}` are sorted; remove the marker"
    );
}

/// Cuts two long texts down to the place where they first differ, with some text before
/// and after it. A report of two long answers is then readable.
fn at_first_difference(php: &str, rust: &str) -> (String, String) {
    const AROUND: usize = 240;
    if php.len().max(rust.len()) <= 2 * AROUND {
        return (php.to_owned(), rust.to_owned());
    }
    let same = php
        .chars()
        .zip(rust.chars())
        .take_while(|(left, right)| left == right)
        .count();
    let cut = |text: &str| {
        let part: String = text
            .chars()
            .skip(same.saturating_sub(AROUND / 2))
            .take(AROUND)
            .collect();
        format!("[from character {same}] ...{part}...")
    };
    (cut(php), cut(rust))
}
