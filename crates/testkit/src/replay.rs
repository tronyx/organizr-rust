//! Replays recorded answers of the PHP application against the Rust application.
//!
//! `parity/recorder/record.py` writes the cases to `parity/cases/<group>.json`. A replay
//! sends each recorded request to the router and compares four aspects of the answer:
//! the status code, the recorded headers, the names of the cookies set, and the body, byte
//! for byte.
//!
//! An aspect may differ from the recording in two ways only:
//!
//! - an **expectation edit**: the expected body is changed before the comparison. An edit
//!   names an approved entry of `docs/deviations.md`, or a difference of the environment
//!   the test runs in.
//! - a **pending marker**: the aspect, or the whole case, is not implemented yet. A pending
//!   aspect must differ. When it starts to match, the test fails until the marker is
//!   removed, so a marker cannot outlive its reason.

use std::collections::BTreeMap;
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
    /// The names of the cookies the answer sets.
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
        }
    }

    /// Expects another value in the body of a case, because of an approved deviation.
    ///
    /// `id` must be an entry of `docs/deviations.md`. `pointer` is a JSON pointer into the
    /// recorded body, for example `/response/data/php`.
    #[must_use]
    pub fn deviation(self, id: &str, case: &str, pointer: &str, value: Value) -> Self {
        let approved = std::fs::read_to_string(repository_root().join("docs/deviations.md"))
            .expect("docs/deviations.md can be read");
        assert!(
            approved.contains(&format!("| {id} |")),
            "{id} is not an entry of docs/deviations.md; a deviation needs approval first"
        );
        self.edit(case, pointer, value)
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
        for case in &recording.cases {
            let expected = self.expected(case);
            let actual = Answer::of(app, &recording, case).await;
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

        let mut cookies = case.response.cookies_set.clone();
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

/// The compared parts of an answer, recorded or actual.
#[derive(Debug)]
struct Answer {
    status: u16,
    /// The recorded headers that are present, by lower-case name.
    headers: BTreeMap<String, String>,
    /// The names of the cookies set, sorted.
    cookies: Vec<String>,
    body: String,
}

impl Answer {
    /// Sends the request of a case to the application and takes the answer apart.
    async fn of(app: &Router, recording: &Recording, case: &Case) -> Self {
        let mut request = Request::builder()
            .method(case.request.method.as_str())
            .uri(&case.request.path)
            .header(header::HOST, &recording.source.host);
        let body = match &case.request.json {
            Some(json) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(json).expect("the request body can be written"))
            }
            None => Body::empty(),
        };
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
        let mut cookies: Vec<String> = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter_map(|cookie| cookie.split_once('=').map(|(name, _)| name.to_owned()))
            .collect();
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
    json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct RecordedResponse {
    status: u16,
    headers: BTreeMap<String, String>,
    cookies_set: Vec<String>,
    body: String,
}

/// The root of the repository, two levels above this crate.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
