"""Record what the PHP application answers, as the ground truth for the parity tests.

Run from the repository root, against a fresh reference container:

    docker compose -f parity/oracle/compose.yml up --build --detach
    python parity/recorder/record.py
    docker compose -f parity/oracle/compose.yml down --volumes

The recorder first records a few answers of the empty install. Then it runs the set-up
wizard with made-up values and records the other groups. Each group becomes one file in
parity/cases/. Recording again must give the same files, so values that change on every
request (the device id, the caller address) are replaced by placeholders.

A case is defined as (name, method, path) or (name, method, path, options). The options:

    json     a value that is sent as a JSON body
    body     a text that is sent as the body, as it is (name the type in `headers`)
    headers  request headers
    cookies  cookies that are sent with fixed values
    jar      the name of a cookie jar. The request carries the cookies of the jar, and
             the cookies of the answer go into it, as in a browser. This is how a later
             case uses the session of an earlier login. The values in a jar are not
             recorded: a replay fills its own jar from the answers it gets.

For each cookie an answer sets, the recorder keeps the name, the domain, the path and
the lifetime: `deleted` when the cookie expires in the past, `session` when it has no
expiry, otherwise the seconds until the expiry, rounded to a whole minute.

Only the Python standard library is used. No data of a real install is involved.
"""

import http.client
import json
import os
import re
import sys
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from urllib.parse import urlsplit

ORACLE_URL = os.environ.get("ORGANIZR_ORACLE_URL", "http://127.0.0.1:8098")
CASES_DIRECTORY = Path(__file__).resolve().parent.parent / "cases"

# What the reference runs. Keep in step with parity/oracle/Dockerfile.
ORGANIZR_VERSION = "2.1.5000"
ORGANIZR_COMMIT = "2c1d41997f6ed61b5a760412efef8a7386a68e62"

# Every request names this host, so recorded answers do not depend on the port in use.
HOST = "organizr.test"

# The made-up install. These values protect nothing: the reference is a throwaway container.
SEED = {
    "driver": "sqlite3",
    "dbName": "parity",
    "dbPath": "/var/www/html/data/parity-db/",
    "license": "personal",
    "hashKey": "parity-hash-key-0123456789",
    "api": "parityapikey01234567",
    "registrationPassword": "parity-registration",
    "username": "admin",
    "password": "parity-admin-password",
    "email": "admin@example.com",
}

# Response headers that are part of the contract. Others (Date, Server) are not recorded.
RECORDED_HEADERS = (
    "content-type",
    "location",
    "cache-control",
    "pragma",
    "expires",
    "x-organizr-user",
    "x-organizr-email",
    "x-organizr-group",
)

# Values that differ on every request, and the placeholder that replaces each one.
REDACTIONS = (
    (re.compile(r'("uuid": ")[0-9a-f-]{36}(")'), r"\1<uuid>\2"),
    (re.compile(r'("(?:user_)?ip": ")[0-9a-fA-F.:]+(")'), r"\1<ip>\2"),
)

# The session cookie carries the random id of the install in its name.
SESSION_COOKIE = re.compile(r"^organizr_token_[0-9a-f-]{36}$")

NOT_FOUND_MESSAGE = "Endpoint Not Found or Defined"

# Answers of an install that has not been set up.
FRESH_INSTALL = [
    ("status", "GET", "/api/v2/status"),
]

# Routes that need no login and no data.
BASICS = [
    ("status", "GET", "/api/v2/status"),
    ("status_with_trailing_slash", "GET", "/api/v2/status/"),
    ("ping", "GET", "/api/v2/ping"),
    ("api_root", "GET", "/api/v2"),
    ("api_root_with_trailing_slash", "GET", "/api/v2/"),
    ("unknown_route", "GET", "/api/v2/nothing/here"),
    ("wrong_method", "POST", "/api/v2/status"),
    ("rox", "GET", "/api/v2/rox"),
    ("roxinsocks_redirect", "GET", "/api/v2/roxinsocks"),
]

# The name and the password of the made-up admin account.
ADMIN = {"username": SEED["username"], "password": SEED["password"]}
FORM = {"Content-Type": "application/x-www-form-urlencoded"}

# Logging in with a local password. No case shares cookies with another one.
LOGIN = [
    ("wrong_password", "POST", "/api/v2/login", {"json": {**ADMIN, "password": "not-it"}}),
    ("unknown_user", "POST", "/api/v2/login", {"json": {**ADMIN, "username": "nobody"}}),
    ("correct_password", "POST", "/api/v2/login", {"json": ADMIN}),
    ("user_name_in_upper_case", "POST", "/api/v2/login", {"json": {**ADMIN, "username": "ADMIN"}}),
    (
        "email_in_another_letter_case",
        "POST",
        "/api/v2/login",
        {"json": {**ADMIN, "username": "Admin@Example.COM"}},
    ),
    ("spaces_around_the_user_name", "POST", "/api/v2/login", {"json": {**ADMIN, "username": " admin "}}),
    # "Remember me": any value other than null counts, also false.
    ("remember_true", "POST", "/api/v2/login", {"json": {**ADMIN, "remember": True}}),
    ("remember_false", "POST", "/api/v2/login", {"json": {**ADMIN, "remember": False}}),
    ("remember_null", "POST", "/api/v2/login", {"json": {**ADMIN, "remember": None}}),
    (
        "other_fields_are_ignored",
        "POST",
        "/api/v2/login",
        {"json": {**ADMIN, "formKey": "anything", "tfaCode": "", "output": True}},
    ),
    (
        "device_cookie_and_user_agent_sent",
        "POST",
        "/api/v2/login",
        {
            "json": ADMIN,
            "cookies": {"organizr_user_uuid": "0e3b1d7c-5f0a-4c59-9f3c-2a6d1b8e4f70"},
            "headers": {"User-Agent": "Parity/1.0"},
        },
    ),
    # Bodies that give no name or no password.
    ("no_body", "POST", "/api/v2/login"),
    ("empty_object", "POST", "/api/v2/login", {"json": {}}),
    ("json_array", "POST", "/api/v2/login", {"json": ["admin", SEED["password"]]}),
    ("json_text", "POST", "/api/v2/login", {"json": "admin"}),
    (
        "invalid_json",
        "POST",
        "/api/v2/login",
        {"body": '{"username": "admin", ', "headers": {"Content-Type": "application/json"}},
    ),
    ("password_only", "POST", "/api/v2/login", {"json": {"password": SEED["password"]}}),
    ("user_name_only", "POST", "/api/v2/login", {"json": {"username": "admin"}}),
    ("number_as_user_name", "POST", "/api/v2/login", {"json": {**ADMIN, "username": 123}}),
    ("number_as_password", "POST", "/api/v2/login", {"json": {**ADMIN, "password": 123}}),
    # How the body is read depends on the content type.
    (
        "form_body",
        "POST",
        "/api/v2/login",
        {"body": f"username=admin&password={SEED['password']}&remember=on", "headers": FORM},
    ),
    (
        "form_body_with_charset_and_escapes",
        "POST",
        "/api/v2/login",
        {
            "body": f"username=%61dmin&password={SEED['password'].replace('-', '%2D')}",
            "headers": {"Content-Type": "application/x-www-form-urlencoded; charset=UTF-8"},
        },
    ),
    (
        "form_body_with_wrong_password",
        "POST",
        "/api/v2/login",
        {"body": "username=admin&password=not+it", "headers": FORM},
    ),
    (
        "json_type_in_another_letter_case_with_charset",
        "POST",
        "/api/v2/login",
        {"body": json.dumps(ADMIN), "headers": {"Content-Type": "Application/JSON; charset=utf-8"}},
    ),
    (
        "json_body_without_a_content_type",
        "POST",
        "/api/v2/login",
        {"body": json.dumps(ADMIN)},
    ),
    (
        "json_body_sent_as_a_form",
        "POST",
        "/api/v2/login",
        {"body": json.dumps(ADMIN), "headers": FORM},
    ),
    ("get_instead_of_post", "GET", "/api/v2/login"),
]

# Requests that show which route the PHP router chooses. The Rust router must choose the
# same one. Keep in step with ROUTING_CASES in crates/api/tests/slim_routes.rs.
ROUTING = [
    ("GET", "/api/v2"),
    ("GET", "/api/v2/"),
    ("GET", "/api/v2/status"),
    ("GET", "/api/v2/status/"),
    ("GET", "/api/v2/launch/"),
    ("GET", "/api/v2/auth"),
    ("POST", "/api/v2/auth/"),
    ("GET", "/api/v2/auth/1"),
    ("GET", "/api/v2/auth/1/allow/10.0.0.1,10.0.0.2"),
    ("PUT", "/api/v2/organizr-auth/2/deny"),
    ("GET", "/api/v2/auth-"),
    ("GET", "/api/v2/auth-5"),
    ("GET", "/api/v2/auth-Plex/"),
    ("GET", "/api/v2/socks/sonarr/api/v3/queue"),
    ("DELETE", "/api/v2/socks/sonarr/"),
    ("POST", "/api/v2/multiple/socks/radarr/2/api/v3/command"),
    ("GET", "/api/v2/socks/sonarr"),
    ("GET", "/api/v2/homepage/donate"),
    ("GET", "/api/v2/homepage/donate/"),
    ("GET", "/api/v2/homepage/donate/5"),
    ("GET", "/api/v2/homepage/donate/success"),
    ("POST", "/api/v2/homepage/jackett/download/"),
    ("POST", "/api/v2/homepage/jackett/download"),
    ("GET", "/api/v2/homepage/jackett/download"),
    ("GET", "/api/v2/log"),
    ("GET", "/api/v2/log/2/abc"),
    ("DELETE", "/api/v2/log/2"),
    ("DELETE", "/api/v2/log/2/abc"),
    ("GET", "/api/v2/config/search/plex"),
    ("GET", "/api/v2/plugins/invites/settings"),
    ("GET", "/api/v2/plugins/invites/ABC123"),
    ("POST", "/api/v2/homepage/overseerr/requests/tv/5/available"),
    ("POST", "/api/v2/homepage/overseerr/requests/tv/5"),
    ("GET", "/api/v2/homepage/overseerr/requests/tv/5"),
    ("POST", "/api/v2/status"),
    ("GET", "/api/v2/login"),
    ("GET", "/api/v2/nothing/here"),
]


# The cookie jars of the recording, by name: cookie name to value.
JARS = {}

# How PHP writes the expiry of a cookie: its own cookies, and the one of the PHP session.
EXPIRY_FORMATS = ("%a, %d-%b-%Y %H:%M:%S GMT", "%a, %d %b %Y %H:%M:%S GMT")


def describe_cookie(header, now):
    """Take a Set-Cookie value apart: (name, value, what the recording keeps)."""
    first, *rest = [part.strip() for part in header.split(";")]
    name, _, value = first.partition("=")
    attributes = {}
    for part in rest:
        key, _, attribute = part.partition("=")
        attributes[key.lower()] = attribute

    lifetime = "session"
    if "expires" in attributes:
        expires = None
        for expiry_format in EXPIRY_FORMATS:
            try:
                expires = datetime.strptime(attributes["expires"], expiry_format)
            except ValueError:
                continue
            break
        if expires is None:
            sys.exit(f"The expiry of a cookie has an unknown form: {header}")
        seconds = (expires.replace(tzinfo=timezone.utc) - now).total_seconds()
        lifetime = "deleted" if seconds < 0 else f"{round(seconds / 60) * 60}s"

    recorded = {
        "name": SESSION_COOKIE.sub("organizr_token_<uuid>", name),
        "domain": attributes.get("domain"),
        "path": attributes.get("path"),
        "lifetime": lifetime,
    }
    return name, value, recorded


def send(method, path, json_body=None, body=None, headers=None, cookies=None, jar=None):
    """Send one request to the reference and return the parts of the answer that matter."""
    target = urlsplit(ORACLE_URL)
    connection = http.client.HTTPConnection(target.hostname, target.port or 80, timeout=60)
    all_headers = {"Host": HOST}
    payload = None
    if json_body is not None:
        payload = json.dumps(json_body).encode("utf-8")
        all_headers["Content-Type"] = "application/json"
    if body is not None:
        payload = body.encode("utf-8")
    all_headers.update(headers or {})
    sent_cookies = dict(cookies or {})
    if jar is not None:
        sent_cookies.update(JARS.setdefault(jar, {}))
    if sent_cookies:
        all_headers["Cookie"] = "; ".join(f"{name}={value}" for name, value in sent_cookies.items())
    try:
        connection.request(method, path, body=payload, headers=all_headers)
        response = connection.getresponse()
        text = response.read().decode("utf-8")
        now = parsedate_to_datetime(response.getheader("Date"))
        recorded_headers = {}
        recorded_cookies = []
        for name, value in response.getheaders():
            lower = name.lower()
            if lower in RECORDED_HEADERS:
                recorded_headers[lower] = value
            elif lower == "set-cookie":
                cookie_name, cookie_value, recorded = describe_cookie(value, now)
                recorded_cookies.append(recorded)
                if jar is not None:
                    if recorded["lifetime"] == "deleted":
                        JARS[jar].pop(cookie_name, None)
                    else:
                        JARS[jar][cookie_name] = cookie_value
        status = response.status
    finally:
        connection.close()

    redacted = []
    for pattern, replacement in REDACTIONS:
        text, count = pattern.subn(replacement, text)
        if count:
            redacted.append(replacement.split("<")[1].split(">")[0])

    request = {"method": method, "path": path}
    if json_body is not None:
        request["json"] = json_body
    if body is not None:
        request["body"] = body
    if headers:
        request["headers"] = headers
    if cookies:
        request["cookies"] = cookies
    if jar is not None:
        request["jar"] = jar
    answer = {
        "status": status,
        "headers": recorded_headers,
        "cookies_set": recorded_cookies,
        "body": text,
    }
    if redacted:
        answer["redacted"] = redacted
    return {"request": request, "response": answer}


def envelope(case):
    """The `response` object of a recorded JSON answer, or an empty dict."""
    try:
        return json.loads(case["response"]["body"]).get("response", {})
    except (ValueError, AttributeError):
        return {}


def record(group, cases, php_version):
    """Write one group of recorded cases to parity/cases/<group>.json."""
    document = {
        "source": {
            "organizr_version": ORGANIZR_VERSION,
            "organizr_commit": ORGANIZR_COMMIT,
            "php_version": php_version,
            "host": HOST,
            "recorded_headers": list(RECORDED_HEADERS),
        },
        "cases": cases,
    }
    CASES_DIRECTORY.mkdir(parents=True, exist_ok=True)
    path = CASES_DIRECTORY / f"{group}.json"
    with path.open("w", encoding="utf-8", newline="\n") as file:
        json.dump(document, file, indent=2, ensure_ascii=False)
        file.write("\n")
    print(f"{path.relative_to(CASES_DIRECTORY.parent.parent)}: {len(cases)} cases")


def named(definitions):
    """Send the requests of a group whose cases have names."""
    cases = []
    for name, method, path, *rest in definitions:
        options = dict(rest[0]) if rest else {}
        if "json" in options:
            options["json_body"] = options.pop("json")
        cases.append({"name": name, **send(method, path, **options)})
    return cases


def main():
    status = envelope(send("GET", "/api/v2/status")).get("data") or {}
    if status.get("action") != "wizard":
        sys.exit(
            "The reference is not a fresh install (status action: "
            f"{status.get('action')!r}). Start a new one:\n"
            "  docker compose -f parity/oracle/compose.yml down --volumes\n"
            "  docker compose -f parity/oracle/compose.yml up --build --detach"
        )
    php_version = status.get("php", "unknown")

    fresh = named(FRESH_INSTALL)
    wizard = send("POST", "/api/v2/wizard", json_body=SEED)
    if envelope(wizard).get("result") != "success":
        sys.exit(f"The set-up wizard failed:\n{wizard['response']['body']}")
    fresh.append({"name": "wizard", **wizard})
    record("fresh_install", fresh, php_version)

    record("basics", named(BASICS), php_version)
    record("login", named(LOGIN), php_version)

    routing = []
    for method, path in ROUTING:
        case = {"name": f"{method} {path}", **send(method, path)}
        answer = envelope(case)
        # What the Rust router is compared with: did the request reach a route at all?
        case["reached_a_route"] = not (
            case["response"]["status"] == 404 and answer.get("message") == NOT_FOUND_MESSAGE
        )
        routing.append(case)
    record("routing", routing, php_version)


if __name__ == "__main__":
    main()
