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

A path and a body can hold `{session_token:<jar>}`, which stands for the session token
in that jar. The name `{session_cookie}` in `cookies` stands for the name of the session
cookie, which differs for every install. The recording keeps the placeholders, and a
replay fills in its own values.

For each cookie an answer sets, the recorder keeps the name, the domain, the path and
the lifetime: `deleted` when the cookie expires in the past, `session` when it has no
expiry, otherwise the seconds until the expiry, rounded to a whole minute.

In a body, the text values of some keys change with every recording: ids, addresses,
tokens and times. REDACTED_KEYS lists them with the placeholder that replaces each value.
A replay replaces the same values in the answers of the Rust application.

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
# There is no `driver`: PHP then uses SQLite, and the Rust application, which checks a
# `driver` against the database it runs on, can replay the request on every engine.
SEED = {
    # The Rust application asks for a set-up code. PHP does not read the field.
    "setupCode": "PARK-TYPA-RKTY",
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

# Keys of a JSON body whose text value differs on every recording, and the placeholder
# that replaces the value. A value that is empty, or not text, stays as it is.
REDACTED_KEYS = {
    # The device id, and the id of the install.
    "uuid": "<uuid>",
    # The address of the caller: here, the gateway of the container network.
    "ip": "<ip>",
    "user_ip": "<ip>",
    # The first characters of a hash of the address of the caller.
    "uid": "<uid>",
    # A session token.
    "token": "<token>",
    # Times: of a token (`date`), and of a stored session.
    "date": "<time>",
    "created": "<time>",
    "expires": "<time>",
}
REDACTIONS = tuple(
    (key, re.compile(r'("%s": ")[^"\\]+(")' % re.escape(key)), placeholder)
    for key, placeholder in REDACTED_KEYS.items()
)

# The session cookie carries the random id of the install in its name.
SESSION_COOKIE = re.compile(r"^organizr_token_[0-9a-f-]{36}$")

# Stands for the name of the session cookie in the `cookies` of a case.
SESSION_COOKIE_PLACEHOLDER = "{session_cookie}"

# Stands for the session token of a jar in a path or a body.
SESSION_TOKEN_PLACEHOLDER = re.compile(r"\{session_token:([a-z_]+)\}")

NOT_FOUND_MESSAGE = "Endpoint Not Found or Defined"

# Answers of an install that has not been set up, and the set-up itself.
FRESH_INSTALL = [
    ("status", "GET", "/api/v2/status"),
    # A field that both applications need is missing or empty. Nothing is set up.
    ("wizard_without_license", "POST", "/api/v2/wizard", {"json": {**SEED, "license": ""}}),
    ("wizard_without_api_key", "POST", "/api/v2/wizard", {"json": {**SEED, "api": ""}}),
    (
        "wizard_without_registration_password",
        "POST",
        "/api/v2/wizard",
        {"json": {**SEED, "registrationPassword": None}},
    ),
    ("wizard_without_user_name", "POST", "/api/v2/wizard", {"json": {**SEED, "username": ""}}),
    ("wizard_without_password", "POST", "/api/v2/wizard", {"json": {**SEED, "password": ""}}),
    (
        "wizard_without_email",
        "POST",
        "/api/v2/wizard",
        {"json": {key: value for key, value in SEED.items() if key != "email"}},
    ),
    ("wizard_with_several_fields_missing", "POST", "/api/v2/wizard", {"json": {**SEED, "email": "", "api": ""}}),
    ("status_before_the_set_up", "GET", "/api/v2/status"),
    ("wizard", "POST", "/api/v2/wizard", {"json": SEED, "jar": "admin"}),
    ("status_after_the_set_up", "GET", "/api/v2/status"),
    ("me_after_the_set_up", "GET", "/api/v2/token/me", {"jar": "admin"}),
    ("wizard_again", "POST", "/api/v2/wizard", {"json": {**SEED, "username": "intruder"}}),
    ("wizard_again_without_fields", "POST", "/api/v2/wizard", {"json": {}}),
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

BROWSER_A = {"User-Agent": "Parity-A/1.0"}
BROWSER_B = {"User-Agent": "Parity-B/2.0"}
NO_TOKEN = {SESSION_COOKIE_PLACEHOLDER: "this-is-no-token"}

# Who the caller is, and the sessions of a user. The cases are one story, in order: the
# set-up wizard has logged the admin in (session 1), then the admin logs in from two
# browsers (sessions 2 and 3).
SESSION = [
    ("me_as_guest", "GET", "/api/v2/token/me"),
    ("login_in_browser_a", "POST", "/api/v2/login", {"json": ADMIN, "jar": "a", "headers": BROWSER_A}),
    ("login_in_browser_b", "POST", "/api/v2/login", {"json": ADMIN, "jar": "b", "headers": BROWSER_B}),
    ("me_in_browser_a", "GET", "/api/v2/token/me", {"jar": "a", "headers": BROWSER_A}),
    ("me_with_a_trailing_slash", "GET", "/api/v2/token/me/", {"jar": "a", "headers": BROWSER_A}),
    # A session cookie that names no session is dropped, on every route.
    ("me_with_a_cookie_that_is_no_token", "GET", "/api/v2/token/me", {"cookies": NO_TOKEN}),
    ("ping_with_a_cookie_that_is_no_token", "GET", "/api/v2/ping", {"cookies": NO_TOKEN}),
    ("unknown_route_with_a_cookie_that_is_no_token", "GET", "/api/v2/nothing/here", {"cookies": NO_TOKEN}),
    ("me_with_an_empty_cookie", "GET", "/api/v2/token/me", {"cookies": {SESSION_COOKIE_PLACEHOLDER: ""}}),
    # Asking whether a token is valid. PHP reads `Token` from the query or from a form.
    ("validate_token_in_the_query", "POST", "/api/v2/token/validate?Token={session_token:a}"),
    (
        "validate_token_in_a_form",
        "POST",
        "/api/v2/token/validate",
        {"body": "Token={session_token:a}", "headers": FORM},
    ),
    (
        "validate_token_of_another_browser",
        "POST",
        "/api/v2/token/validate?Token={session_token:a}",
        {"jar": "b", "headers": BROWSER_B},
    ),
    ("validate_text_that_is_no_token", "POST", "/api/v2/token/validate?Token=this-is-no-token"),
    ("validate_empty_token", "POST", "/api/v2/token/validate?Token="),
    ("validate_with_get", "GET", "/api/v2/token/validate?Token={session_token:a}"),
    # Deleting sessions. Only a logged-in user can, and only sessions of their own.
    ("delete_as_guest", "DELETE", "/api/v2/token/3"),
    ("delete_id_zero", "DELETE", "/api/v2/token/0", {"jar": "a", "headers": BROWSER_A}),
    ("delete_unknown_id", "DELETE", "/api/v2/token/99999", {"jar": "a", "headers": BROWSER_A}),
    ("delete_id_that_is_no_number", "DELETE", "/api/v2/token/abc", {"jar": "a", "headers": BROWSER_A}),
    # The paths of the two other token routes are ids for a DELETE.
    ("delete_id_me", "DELETE", "/api/v2/token/me", {"jar": "a", "headers": BROWSER_A}),
    ("delete_id_validate", "DELETE", "/api/v2/token/validate", {"jar": "a", "headers": BROWSER_A}),
    ("delete_id_me_as_guest", "DELETE", "/api/v2/token/me"),
    ("delete_without_an_id", "DELETE", "/api/v2/token/", {"jar": "a", "headers": BROWSER_A}),
    ("delete_session_of_browser_b", "DELETE", "/api/v2/token/3", {"jar": "a", "headers": BROWSER_A}),
    ("me_in_browser_a_after_the_delete", "GET", "/api/v2/token/me", {"jar": "a", "headers": BROWSER_A}),
    ("validate_token_of_the_deleted_session", "POST", "/api/v2/token/validate?Token={session_token:b}"),
    # The cookie of browser B is dropped twice: once because its session is gone, once
    # by the logout itself.
    ("logout_in_browser_b_after_the_delete", "GET", "/api/v2/logout", {"jar": "b", "headers": BROWSER_B}),
    ("me_in_browser_b_after_the_logout", "GET", "/api/v2/token/me", {"jar": "b", "headers": BROWSER_B}),
    ("delete_own_session", "DELETE", "/api/v2/token/2", {"jar": "a", "headers": BROWSER_A}),
    ("me_in_browser_a_after_deleting_the_own_session", "GET", "/api/v2/token/me", {"jar": "a", "headers": BROWSER_A}),
]

# Logging out. The cases with the jar `browser` are one visit: log in, log out, log out
# again. A logout with a session cookie that is no longer valid is in SESSION.
LOGOUT = [
    ("login", "POST", "/api/v2/login", {"json": ADMIN, "jar": "browser"}),
    ("logout", "GET", "/api/v2/logout", {"jar": "browser"}),
    ("logout_again", "GET", "/api/v2/logout", {"jar": "browser"}),
    ("logout_without_cookies", "GET", "/api/v2/logout"),
    # Cookies of other applications that a login can set. PHP drops every cookie whose
    # name contains `tautulli` or `user-`.
    (
        "logout_with_cookies_of_other_applications",
        "GET",
        "/api/v2/logout",
        {
            "cookies": {
                "organizr_user_uuid": "0e3b1d7c-5f0a-4c59-9f3c-2a6d1b8e4f70",
                "tautulli_token_abc": "1",
                "my_tautulli": "2",
                "Tautulli": "3",
                "user-abc": "4",
                "superuser-1": "5",
                "user": "6",
                "mpt": "7",
                "theme": "8",
            }
        },
    ),
    ("post_instead_of_get", "POST", "/api/v2/logout"),
]

# The limit on failed logins. PHP keeps no count on the server: its front end counts the
# logins of the page in `loginAttempts`, starting at 1, and PHP remembers a lock-out in
# the cookie `lockout`. The cases are one visit of a browser that behaves like that front
# end. The Rust application counts on the server and must give the same answers.
WRONG = {**ADMIN, "password": "not-it"}
LOCKOUT = [
    ("wrong_password_1", "POST", "/api/v2/login", {"json": {**WRONG, "loginAttempts": 1}, "jar": "visitor"}),
    ("wrong_password_2", "POST", "/api/v2/login", {"json": {**WRONG, "loginAttempts": 2}, "jar": "visitor"}),
    ("wrong_password_3", "POST", "/api/v2/login", {"json": {**WRONG, "loginAttempts": 3}, "jar": "visitor"}),
    ("wrong_password_4", "POST", "/api/v2/login", {"json": {**WRONG, "loginAttempts": 4}, "jar": "visitor"}),
    # The failure that reaches the limit of 5 already gets the lock-out answer.
    ("wrong_password_5", "POST", "/api/v2/login", {"json": {**WRONG, "loginAttempts": 5}, "jar": "visitor"}),
    (
        "correct_password_while_locked_out",
        "POST",
        "/api/v2/login",
        {"json": {**ADMIN, "loginAttempts": 6}, "jar": "visitor"},
    ),
    # A new page starts its count again, but the lock-out stays.
    (
        "correct_password_while_locked_out_from_a_new_page",
        "POST",
        "/api/v2/login",
        {"json": {**ADMIN, "loginAttempts": 1}, "jar": "visitor"},
    ),
    ("other_routes_work_while_locked_out", "GET", "/api/v2/token/me", {"jar": "visitor"}),
]

# The API key of the install: in the header `token`, or else in the query field `apikey`.
# It gives the access of an Admin. `GET /token/me` shows who PHP takes the caller for, and
# `DELETE /token/{id}` needs a caller who is more than a guest.
KEY = SEED["api"]
WRONG_KEY = KEY[:-1] + "8"
API_KEY = [
    ("me_with_the_key_in_the_header", "GET", "/api/v2/token/me", {"headers": {"Token": KEY}}),
    ("me_with_the_key_in_the_query", "GET", f"/api/v2/token/me?apikey={KEY}"),
    ("me_with_another_header_name_case", "GET", "/api/v2/token/me", {"headers": {"TOKEN": KEY}}),
    ("me_with_a_wrong_key_in_the_header", "GET", "/api/v2/token/me", {"headers": {"Token": WRONG_KEY}}),
    ("me_with_a_wrong_key_in_the_query", "GET", f"/api/v2/token/me?apikey={WRONG_KEY}"),
    ("me_with_a_part_of_the_key", "GET", "/api/v2/token/me", {"headers": {"Token": KEY[:19]}}),
    ("me_with_the_key_and_more", "GET", "/api/v2/token/me", {"headers": {"Token": KEY + "8"}}),
    ("me_with_the_key_in_upper_case", "GET", "/api/v2/token/me", {"headers": {"Token": KEY.upper()}}),
    ("me_with_another_query_field_case", "GET", f"/api/v2/token/me?APIKEY={KEY}"),
    # The header counts when it is sent, also when only the query has the key.
    (
        "me_with_a_wrong_header_and_the_key_in_the_query",
        "GET",
        f"/api/v2/token/me?apikey={KEY}",
        {"headers": {"Token": WRONG_KEY}},
    ),
    # A login, so that a replay knows the name of the session cookie for the next case.
    ("login", "POST", "/api/v2/login", {"json": ADMIN, "jar": "admin"}),
    (
        "me_with_the_key_and_a_cookie_that_is_no_token",
        "GET",
        "/api/v2/token/me",
        {"headers": {"Token": KEY}, "cookies": NO_TOKEN},
    ),
    # A route that a guest may not use. No session has the id, so nothing is deleted.
    ("delete_without_the_key", "DELETE", "/api/v2/token/99999"),
    ("delete_with_the_key_in_the_header", "DELETE", "/api/v2/token/99999", {"headers": {"Token": KEY}}),
    ("delete_with_the_key_in_the_query", "DELETE", f"/api/v2/token/99999?apikey={KEY}"),
    ("delete_with_a_wrong_key", "DELETE", "/api/v2/token/99999", {"headers": {"Token": WRONG_KEY}}),
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
JARS: dict[str, dict[str, str]] = {}

# The name of the session cookie of this install, once an answer has set the cookie.
session_cookie_name = None


def fill_in(text):
    """Replace each `{session_token:<jar>}` with the session token of that jar."""

    def token_of(match):
        jar = JARS.get(match.group(1), {})
        if session_cookie_name not in jar:
            sys.exit(f"The jar `{match.group(1)}` holds no session token.")
        return jar[session_cookie_name]

    return SESSION_TOKEN_PLACEHOLDER.sub(token_of, text)

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

    if SESSION_COOKIE.match(name):
        global session_cookie_name
        session_cookie_name = name
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
        payload = fill_in(body).encode("utf-8")
    all_headers.update(headers or {})
    sent_cookies = {}
    for name, value in (cookies or {}).items():
        if name == SESSION_COOKIE_PLACEHOLDER:
            if session_cookie_name is None:
                sys.exit("No answer has set the session cookie yet, so its name is not known.")
            name = session_cookie_name
        sent_cookies[name] = value
    if jar is not None:
        sent_cookies.update(JARS.setdefault(jar, {}))
    if sent_cookies:
        all_headers["Cookie"] = "; ".join(f"{name}={value}" for name, value in sent_cookies.items())
    try:
        connection.request(method, fill_in(path), body=payload, headers=all_headers)
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
    for key, pattern, placeholder in REDACTIONS:
        text, count = pattern.subn(lambda match: match.group(1) + placeholder + match.group(2), text)
        if count:
            redacted.append(key)

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
            "redacted_keys": REDACTED_KEYS,
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
    # Every group starts with empty jars, as a replay of the group does.
    JARS.clear()
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
    wizard = next(case for case in fresh if case["name"] == "wizard")
    if envelope(wizard).get("result") != "success":
        sys.exit(f"The set-up wizard failed:\n{wizard['response']['body']}")
    record("fresh_install", fresh, php_version)

    record("basics", named(BASICS), php_version)
    # Before the other groups log in, so the only earlier session is the one of the wizard.
    record("session", named(SESSION), php_version)
    record("login", named(LOGIN), php_version)
    record("logout", named(LOGOUT), php_version)
    record("lockout", named(LOCKOUT), php_version)
    record("api_key", named(API_KEY), php_version)

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
