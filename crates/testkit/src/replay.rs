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
//! earlier login of the same replay.
//!
//! An aspect may differ from the recording in two ways only:
//!
//! - an **expectation edit**: the expected body is changed, or an expected cookie is
//!   removed, before the comparison. An edit names an approved entry of
//!   `docs/deviations.md`, or a difference of the environment the test runs in.
//! - a **pending marker**: the aspect, or the whole case, is not implemented yet. A pending
//!   aspect must differ. When it starts to match, the test fails until the marker is
//!   removed, so a marker cannot outlive its reason.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::PathBuf;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, header};
use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::Value;
use tower::ServiceExt;

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
    pending_aspects: Vec<(String, Aspect)>,
    pending_cases: Vec<String>,
    absent_cookies: Vec<String>,
}

#[derive(Debug)]
struct Edit {
    case: String,
    pointer: String,
    value: Value,
}

impl Replay {
    /// Prepares the replay of `parity/cases/<group>.json`.
    pub fn group(group: &str) -> Self {
        Self {
            group: group.to_owned(),
            edits: Vec::new(),
            pending_aspects: Vec::new(),
            pending_cases: Vec::new(),
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
        let mut jars = Jars::new();
        for case in &recording.cases {
            let expected = self.expected(case);
            let actual = Answer::of(app, &recording, case, &mut jars).await;
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
                    let _ = writeln!(
                        problems,
                        "case `{}`: {aspect:?} differs\n  PHP:  {}\n  Rust: {}",
                        case.name,
                        expected.show(aspect),
                        actual.show(aspect)
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

    /// The recorded answer of a case, with the expectation edits applied.
    fn expected(&self, case: &Case) -> Answer {
        let mut body = case.response.body.clone();
        let edits: Vec<&Edit> = self
            .edits
            .iter()
            .filter(|edit| edit.case == case.name)
            .collect();
        if !edits.is_empty() {
            let mut json: Value =
                serde_json::from_str(&body).expect("a case with edits has a JSON body");
            for edit in edits {
                let target = json.pointer_mut(&edit.pointer).unwrap_or_else(|| {
                    panic!(
                        "case `{}`: the recorded body has nothing at `{}`",
                        case.name, edit.pointer
                    )
                });
                *target = edit.value.clone();
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

    /// Fails when an edit or a marker names a case the group does not have.
    fn check_names(&self, recording: &Recording) {
        let named = self
            .edits
            .iter()
            .map(|edit| &edit.case)
            .chain(self.pending_aspects.iter().map(|(case, _)| case))
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

/// The cookie jars of a replay, by name: cookie name to value.
type Jars = HashMap<String, BTreeMap<String, String>>;

impl Answer {
    /// Sends the request of a case to the application and takes the answer apart.
    async fn of(app: &Router, recording: &Recording, case: &Case, jars: &mut Jars) -> Self {
        let mut request = Request::builder()
            .method(case.request.method.as_str())
            .uri(&case.request.path)
            .header(header::HOST, &recording.source.host);
        let body = match (&case.request.json, &case.request.body) {
            (Some(json), _) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(json).expect("the request body can be written"))
            }
            (None, Some(text)) => Body::from(text.clone()),
            (None, None) => Body::empty(),
        };
        for (name, value) in &case.request.headers {
            request = request.header(name.as_str(), value.as_str());
        }
        let mut sent_cookies = case.request.cookies.clone();
        if let Some(jar) = &case.request.jar {
            sent_cookies.extend(jars.entry(jar.clone()).or_default().clone());
        }
        if !sent_cookies.is_empty() {
            let cookies: Vec<String> = sent_cookies
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            request = request.header(header::COOKIE, cookies.join("; "));
        }
        let response = app
            .clone()
            .oneshot(request.body(body).expect("the recorded request is valid"))
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
            if let Some(jar) = &case.request.jar {
                let jar = jars.entry(jar.clone()).or_default();
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

        Self {
            status,
            headers,
            cookies,
            body: String::from_utf8(bytes.to_vec()).expect("the body is UTF-8"),
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
                "organizr_token_<uuid>".to_owned()
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
