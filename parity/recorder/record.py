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

import base64
import hashlib
import hmac
import http.client
import json
import os
import re
import struct
import sys
import time
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from urllib.parse import urlsplit
from urllib.request import urlopen

ORACLE_URL = os.environ.get("ORGANIZR_ORACLE_URL", "http://127.0.0.1:8098")
# The mail catcher of parity/oracle/compose.yml: it keeps the e-mails of the reference.
MAIL_CATCHER_URL = os.environ.get("ORGANIZR_MAIL_CATCHER_URL", "http://127.0.0.1:8099")
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
    # The hash of a password, which has a new random part on every recording, and the
    # time at which a user was stored.
    "password": "<password hash>",
    "register_date": "<time>",
    # The secret of a second factor, which is random.
    "secret": "<secret>",
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

# How long the recorder waits after a change of the configuration of PHP. PHP looks at
# a file again two seconds after it last looked, and it does not keep a file that is
# younger than two seconds. A change showed after 0.5 to 3.1 seconds.
CODE_CACHE_SECONDS = 4.5

NOT_FOUND_MESSAGE = "Endpoint Not Found or Defined"

# Answers of an install that has not been set up, and the set-up itself.
FRESH_INSTALL: list[tuple] = [
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
    ("launch_before_the_set_up", "GET", "/api/v2/launch"),
    ("wizard", "POST", "/api/v2/wizard", {"json": SEED, "jar": "admin"}),
    ("status_after_the_set_up", "GET", "/api/v2/status"),
    ("me_after_the_set_up", "GET", "/api/v2/token/me", {"jar": "admin"}),
    # What the front end loads first: the user, the tabs and how the page looks.
    ("launch_as_guest", "GET", "/api/v2/launch"),
    ("launch_as_guest_with_a_trailing_slash", "GET", "/api/v2/launch/"),
    ("launch_as_admin", "GET", "/api/v2/launch", {"jar": "admin"}),
    # The answer also tells the page which cookies of other services the browser holds,
    # which browser it is and which page it came from.
    (
        "launch_with_cookies_of_other_services",
        "GET",
        "/api/v2/launch",
        {
            "headers": {"User-Agent": "Mozilla/5.0 (parity)", "Referer": "http://organizr.test/?from=here"},
            "cookies": {
                "mpt": "plex%20token+one",
                "Auth": "ombi-token",
                "oAuth": "true",
                "connect.sid": "s%3Aoverseerr",
                "petio_jwt": "petio",
                "jellyfin_credentials": "%7B%22Servers%22%3A1%7D",
                "komga_token": "komga",
                "user-12ab-34cd": "jellyfin user",
                "other": "is not sent back",
            },
        },
    ),
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

# Registering an account. REGISTRATION is the registration password of the install; see
# prepare_registration().
REGISTRATION = "let-new-users-in"
ZOE = {
    "username": "zoe",
    "password": "the-password-of-zoe",
    "email": "zoe@example.com",
    "registrationPassword": REGISTRATION,
}
REGISTER = [
    # PHP asks for the fields in this order.
    ("without_fields", "POST", "/api/v2/users/register", {"json": {}}),
    ("without_email", "POST", "/api/v2/users/register", {"json": {**ZOE, "email": ""}}),
    ("without_user_name", "POST", "/api/v2/users/register", {"json": {**ZOE, "username": None}}),
    ("without_password", "POST", "/api/v2/users/register", {"json": {**ZOE, "password": ""}}),
    (
        "without_registration_password",
        "POST",
        "/api/v2/users/register",
        {"json": {key: value for key, value in ZOE.items() if key != "registrationPassword"}},
    ),
    # The text `0` counts as nothing in PHP.
    ("user_name_zero", "POST", "/api/v2/users/register", {"json": {**ZOE, "username": "0"}}),
    (
        "wrong_registration_password",
        "POST",
        "/api/v2/users/register",
        {"json": {**ZOE, "registrationPassword": "let-me-in-anyway"}},
    ),
    # The checks of a new account, in the order of PHP.
    ("email_that_is_no_address", "POST", "/api/v2/users/register", {"json": {**ZOE, "email": "not-an-address"}}),
    ("user_name_taken", "POST", "/api/v2/users/register", {"json": {**ZOE, "username": "ADMIN"}}),
    ("email_taken", "POST", "/api/v2/users/register", {"json": {**ZOE, "email": "Admin@Example.COM"}}),
    ("user_name_too_long", "POST", "/api/v2/users/register", {"json": {**ZOE, "username": "z" * 51}}),
    (
        "email_too_long",
        "POST",
        "/api/v2/users/register",
        {"json": {**ZOE, "email": "z" * 39 + "@example.com"}},
    ),
    ("password_too_long", "POST", "/api/v2/users/register", {"json": {**ZOE, "password": "p" * 201}}),
    # Two things are wrong at once: the answer shows which check comes first.
    (
        "no_address_and_user_name_taken",
        "POST",
        "/api/v2/users/register",
        {"json": {**ZOE, "email": "not-an-address", "username": "admin"}},
    ),
    (
        "user_name_taken_and_password_too_long",
        "POST",
        "/api/v2/users/register",
        {"json": {**ZOE, "username": "admin", "password": "p" * 201}},
    ),
    (
        "user_name_and_email_too_long",
        "POST",
        "/api/v2/users/register",
        {"json": {**ZOE, "username": "z" * 51, "email": "z" * 39 + "@example.com", "password": "p" * 201}},
    ),
    (
        "longest_values",
        "POST",
        "/api/v2/users/register",
        {
            "json": {
                **ZOE,
                "username": "y" * 50,
                "email": "y" * 38 + "@example.com",
                "password": "p" * 200,
            }
        },
    ),
    ("new_account", "POST", "/api/v2/users/register", {"json": ZOE, "jar": "zoe"}),
    ("same_account_again", "POST", "/api/v2/users/register", {"json": ZOE}),
    (
        "login_as_the_new_account",
        "POST",
        "/api/v2/login",
        {"json": {"username": "ZOE", "password": ZOE["password"]}},
    ),
    # The space around the name and the address is cut off.
    (
        "space_around_name_and_address",
        "POST",
        "/api/v2/users/register",
        {"json": {**ZOE, "username": "  Yan  ", "email": "  Yan@Example.com  "}},
    ),
    (
        "login_as_the_account_with_the_space_cut_off",
        "POST",
        "/api/v2/login",
        {"json": {"username": "yan@example.com", "password": ZOE["password"]}},
    ),
    (
        "form_body",
        "POST",
        "/api/v2/users/register",
        {
            "body": f"username=form&password=pw&email=form%40example.com&registrationPassword={REGISTRATION}",
            "headers": FORM,
        },
    ),
]

# The tab routes. Every case that passes carries the API key, which counts as the Admin.
# A fresh install has the tabs "Settings" (id 1) and "Homepage" (id 2). The tabs that the
# cases add get the ids from 3 on, in the order of the cases.
AS_ADMIN = {"Token": KEY}
PLEX = {"name": "Plex", "url": "https://plex.example.com/", "image": "plugins/images/tabs/plex.png"}


def two_factor_cases():
    """The second factor: making, checking, saving, logging in with it, and removing it."""
    path = "/api/v2/2fa"
    user = {"jar": "first"}
    login = {**ADMIN}

    def as_user(name, method, to, json_body=None):
        options = dict(user)
        if json_body is not None:
            options["json"] = json_body
        return (name, method, to, options)

    whole = {"secret": "{2fa_secret}", "code": "{2fa_code}", "type": "google"}
    return [
        # Only a user who is logged in can do any of it.
        ("create_as_guest", "POST", path + "/google"),
        ("verify_as_guest", "POST", path, {"json": {"secret": "A", "code": "1", "type": "google"}}),
        ("save_as_guest", "PUT", path, {"json": {"secret": "A", "type": "google"}}),
        ("remove_as_guest", "DELETE", path),
        ("login", "POST", "/api/v2/login", {"json": login, "jar": "first"}),
        # Making a secret.
        as_user("create_of_an_unknown_kind", "POST", path + "/other"),
        as_user("create", "POST", path + "/google"),
        # Checking a code against a secret that is not saved yet.
        as_user("verify_without_a_body", "POST", path),
        as_user("verify_without_a_secret", "POST", path, {"code": "{2fa_code}", "type": "google"}),
        as_user("verify_without_a_code", "POST", path, {"secret": "{2fa_secret}", "type": "google"}),
        as_user("verify_without_a_type", "POST", path, {"secret": "{2fa_secret}", "code": "{2fa_code}"}),
        as_user("verify_an_unknown_kind", "POST", path, {**whole, "type": "other"}),
        as_user("verify_a_wrong_code", "POST", path, {**whole, "code": "{2fa_wrong_code}"}),
        as_user("verify", "POST", path, whole),
        as_user("verify_the_code_of_five_steps_ago", "POST", path, {**whole, "code": "{2fa_code:-5}"}),
        as_user("verify_the_code_of_seven_steps_ago", "POST", path, {**whole, "code": "{2fa_code:-7}"}),
        as_user("verify_the_code_in_five_steps", "POST", path, {**whole, "code": "{2fa_code:+5}"}),
        as_user("verify_the_code_in_seven_steps", "POST", path, {**whole, "code": "{2fa_code:+7}"}),
        # Saving.
        as_user("save_without_a_body", "PUT", path),
        as_user("save_without_a_secret", "PUT", path, {"type": "google"}),
        as_user("save_without_a_type", "PUT", path, {"secret": "{2fa_secret}"}),
        as_user("save", "PUT", path, {"secret": "{2fa_secret}", "type": "google"}),
        # Logging in asks for the code now.
        ("login_without_the_code", "POST", "/api/v2/login", {"json": login, "jar": "second"}),
        ("login_with_an_empty_code", "POST", "/api/v2/login", {"json": {**login, "tfaCode": ""}, "jar": "second"}),
        ("login_with_a_wrong_code", "POST", "/api/v2/login", {"json": {**login, "tfaCode": "{2fa_wrong_code}"}, "jar": "second"}),
        ("login_with_a_wrong_password_and_the_code", "POST", "/api/v2/login", {"json": {**login, "password": "not-it", "tfaCode": "{2fa_code}"}, "jar": "second"}),
        ("login_with_the_code", "POST", "/api/v2/login", {"json": {**login, "tfaCode": "{2fa_code}"}, "jar": "second"}),
        # Removing. The first session is still good.
        as_user("remove", "DELETE", path),
        as_user("remove_again", "DELETE", path),
        ("login_after_the_removal", "POST", "/api/v2/login", {"json": login, "jar": "third"}),
    ]


# The mail plugin. The reference sends to the mail catcher, which needs no login and
# offers no encryption.
MAIL_ON = {
    "PHPMAILER-enabled": True,
    "PHPMAILER-smtpHost": "mailpit",
    "PHPMAILER-smtpHostPort": "1025",
    "PHPMAILER-smtpHostType": "n/a",
    "PHPMAILER-smtpHostAuth": False,
    "PHPMAILER-smtpHostSenderName": "Parity Organizr",
    "PHPMAILER-smtpHostSenderEmail": "organizr@example.com",
    "PHPMAILER-template": "dark",
    "PHPMAILER-logo": "https://example.com/logo.png",
}
MAIL_PATH = "/api/v2/plugins/php-mailer/email"
JO = {"username": "jo", "email": "jo@example.com", "password": "the-password-of-jo"}
LETTER = {
    "bcc": "first@example.com,second@example.com",
    "subject": "News from {title}",
    "body": "<p>Come to {domain} soon.</p>",
}


def mail_cases():
    """Password recovery and the routes of the mail plugin."""
    recover = "/api/v2/users/recover"
    return [
        # While the plugin is off.
        ("recover_without_an_address", "POST", recover, {"json": {}}),
        ("recover_while_mail_is_off", "POST", recover, {"json": {"email": JO["email"]}}),
        ("test_as_guest", "GET", MAIL_PATH + "/test"),
        ("send_as_guest", "POST", MAIL_PATH + "/send", {"json": LETTER}),
        ("list_as_guest", "GET", MAIL_PATH + "/list"),
        admin_case("send_while_mail_is_off", "POST", MAIL_PATH + "/send", LETTER),
        admin_case("list", "GET", MAIL_PATH + "/list"),
        admin_case("add_a_user", "POST", "/api/v2/users", JO),
        admin_case("switch_mail_on", "PUT", "/api/v2/config", MAIL_ON, settle=True),
        # Recovery: the answer does not tell whether the address is known.
        ("recover_an_unknown_address", "POST", recover, {"json": {"email": "nobody@example.com"}}),
        ("recover", "POST", recover, {"json": {"email": JO["email"]}}),
        ("recover_in_other_letter_case", "POST", recover, {"json": {"email": "JO@Example.COM"}}),
        # The plugin routes.
        ("login", "POST", "/api/v2/login", {"json": ADMIN, "jar": "admin"}),
        ("test", "GET", MAIL_PATH + "/test", {"jar": "admin"}),
        admin_case("send", "POST", MAIL_PATH + "/send", LETTER),
        admin_case("send_to_one_address", "POST", MAIL_PATH + "/send", {**LETTER, "bcc": "only@example.com"}),
        admin_case("send_without_a_body", "POST", MAIL_PATH + "/send"),
        admin_case("send_without_recipients", "POST", MAIL_PATH + "/send", {"subject": "S", "body": "B"}),
        admin_case("list_with_the_new_user", "GET", MAIL_PATH + "/list"),
    ]


# The new password in an e-mail of the recovery, which is random.
NEW_PASSWORD = re.compile(r"(Here is your new password: )[^<\s]+")


def record_mails():
    """Write the e-mails that the mail catcher got to parity/cases/mail_messages.json."""
    with urlopen(MAIL_CATCHER_URL + "/api/v1/messages?limit=100", timeout=30) as answer:
        listed = json.load(answer)["messages"]
    mails = []
    # The catcher lists the newest e-mail first.
    for entry in reversed(listed):
        with urlopen(f"{MAIL_CATCHER_URL}/api/v1/message/{entry['ID']}", timeout=30) as answer:
            mail = json.load(answer)
        mails.append(
            {
                "from": mail["From"],
                "reply_to": mail["ReplyTo"],
                "to": mail["To"],
                "bcc": mail["Bcc"],
                "subject": mail["Subject"],
                "html": NEW_PASSWORD.sub(r"\1<password>", mail["HTML"]),
            }
        )
    path = CASES_DIRECTORY / "mail_messages.json"
    document = {
        "source": "The e-mails that Organizr %s sent during the group `mail`, in order" % ORGANIZR_VERSION,
        "mails": mails,
    }
    with path.open("w", encoding="utf-8", newline="\n") as file:
        json.dump(document, file, indent=2, ensure_ascii=False)
        file.write("\n")
    print(f"{path.relative_to(CASES_DIRECTORY.parent.parent)}: {len(mails)} e-mails")


def admin_case(name, method, path, json_body=None, **options):
    """A case that carries the API key, which counts as the Admin."""
    if json_body is not None:
        options["json"] = json_body
    options["headers"] = {**options.get("headers", {}), **AS_ADMIN}
    return (name, method, path, options)


TABS = [
    # Reading.
    ("tabs_as_guest", "GET", "/api/v2/tabs"),
    admin_case("tabs", "GET", "/api/v2/tabs"),
    ("tab_as_guest", "GET", "/api/v2/tabs/1"),
    admin_case("tab", "GET", "/api/v2/tabs/1"),
    admin_case("tab_with_a_leading_zero_in_the_id", "GET", "/api/v2/tabs/01"),
    admin_case("tab_unknown", "GET", "/api/v2/tabs/999"),
    admin_case("tab_id_not_a_number", "GET", "/api/v2/tabs/abc"),
    # Adding: what is refused.
    ("add_as_guest", "POST", "/api/v2/tabs", {"json": PLEX}),
    admin_case("add_without_a_body", "POST", "/api/v2/tabs"),
    admin_case("add_empty_object", "POST", "/api/v2/tabs", {}),
    admin_case("add_json_list", "POST", "/api/v2/tabs", ["Plex"]),
    admin_case("add_unknown_fields_only", "POST", "/api/v2/tabs", {"nothing": 1}),
    admin_case("add_without_name", "POST", "/api/v2/tabs", {"url": "u", "image": "i"}),
    admin_case("add_without_address", "POST", "/api/v2/tabs", {"name": "A", "image": "i"}),
    admin_case("add_without_image", "POST", "/api/v2/tabs", {"name": "A", "url": "u"}),
    # Adding: the least a tab needs (id 3), and names.
    admin_case("add", "POST", "/api/v2/tabs", PLEX),
    admin_case("added_tab", "GET", "/api/v2/tabs/3"),
    admin_case("add_name_taken", "POST", "/api/v2/tabs", PLEX),
    admin_case("add_name_taken_in_another_letter_case", "POST", "/api/v2/tabs", {**PLEX, "name": "PLEX"}),
    # The name is checked before the address.
    admin_case("add_name_taken_without_address", "POST", "/api/v2/tabs", {"name": "plex"}),
    admin_case("add_name_too_long", "POST", "/api/v2/tabs", {**PLEX, "name": "x" * 51}),
    admin_case("add_name_of_50_characters", "POST", "/api/v2/tabs", {**PLEX, "name": "y" * 50}),
    admin_case(
        "add_with_spaces_around_name_and_image",
        "POST",
        "/api/v2/tabs",
        {**PLEX, "name": "  Spaced  ", "image": "  plugins/images/tabs/sonarr.png  "},
    ),
    admin_case("tab_with_spaces_cut_off", "GET", "/api/v2/tabs/5"),
    admin_case("add_with_a_number_as_name", "POST", "/api/v2/tabs", {**PLEX, "name": 5}),
    # The list of tabs fills in the placeholders of an address. One tab alone shows the
    # address as it is stored.
    admin_case(
        "add_with_a_local_address_only",
        "POST",
        "/api/v2/tabs",
        {
            "name": "Local",
            "url_local": "http://{domain}/{username}/{username_lower}/{email}/{group}/{group_id}/{komga}/{other}/{domain}",
            "image": "fontawesome::home",
        },
    ),
    admin_case("tab_with_placeholders", "GET", "/api/v2/tabs/7"),
    # Adding: every field (id 8). The place 7 is taken by the tab before it.
    admin_case(
        "add_with_every_field",
        "POST",
        "/api/v2/tabs",
        {
            "name": "Full",
            "url": "https://full.example.com/",
            "url_local": "http://10.0.0.6/",
            "image": "fontawesome::film",
            "order": 7,
            "category_id": 0,
            "default": 1,
            "enabled": 1,
            "group_id": 4,
            "group_id_max": 1,
            "add_to_admin": 1,
            "type": 2,
            "splash": 1,
            "ping": 1,
            "ping_url": "10.0.0.6:80",
            "timeout": 1,
            "timeout_ms": 60000,
            "preload": 1,
        },
    ),
    admin_case("tab_with_every_field", "GET", "/api/v2/tabs/8"),
    # Adding: other ways to write a value (ids 9 to 12).
    admin_case(
        "add_with_numbers_as_text",
        "POST",
        "/api/v2/tabs",
        {**PLEX, "name": "Texts", "enabled": "1", "group_id": "3", "type": "2", "order": "12", "splash": "0"},
    ),
    admin_case("tab_with_numbers_as_text", "GET", "/api/v2/tabs/9"),
    admin_case(
        "add_with_true_and_false",
        "POST",
        "/api/v2/tabs",
        {**PLEX, "name": "Switches", "enabled": True, "splash": False, "ping": True, "preload": False},
    ),
    admin_case("tab_with_true_and_false", "GET", "/api/v2/tabs/10"),
    admin_case(
        "add_as_form",
        "POST",
        "/api/v2/tabs",
        **{"body": "name=Form&url=https%3A%2F%2Fform.example.com%2F&image=i&enabled=1", "headers": FORM},
    ),
    admin_case("tab_from_a_form", "GET", "/api/v2/tabs/11"),
    admin_case(
        "add_with_null_for_address_and_image",
        "POST",
        "/api/v2/tabs",
        {"name": "Nulls", "url": None, "image": None, "enabled": None, "group_id": None, "order": None},
    ),
    admin_case("tab_with_null_for_address_and_image", "GET", "/api/v2/tabs/12"),
    admin_case("tabs_after_adding", "GET", "/api/v2/tabs"),
    admin_case("tabs_with_a_komga_cookie", "GET", "/api/v2/tabs", cookies={"komga_token": "komga-value"}),
    # A name with characters that HTML gives a meaning (id 13). The tab is deleted again.
    admin_case("add_with_html_in_the_name", "POST", "/api/v2/tabs", {**PLEX, "name": "A & <b>"}),
    admin_case("tab_with_html_in_the_name", "GET", "/api/v2/tabs/13"),
    ("delete_as_guest", "DELETE", "/api/v2/tabs/13"),
    admin_case("delete", "DELETE", "/api/v2/tabs/13"),
    admin_case("delete_again", "DELETE", "/api/v2/tabs/13"),
    admin_case("delete_unknown", "DELETE", "/api/v2/tabs/999"),
    admin_case("delete_id_not_a_number", "DELETE", "/api/v2/tabs/abc"),
    admin_case("deleted_tab", "GET", "/api/v2/tabs/13"),
    # Changing: what is refused.
    ("change_as_guest", "PUT", "/api/v2/tabs/3", {"json": {"name": "Guest"}}),
    admin_case("change_without_a_body", "PUT", "/api/v2/tabs/3"),
    admin_case("change_empty_object", "PUT", "/api/v2/tabs/3", {}),
    admin_case(
        "change_as_form",
        "PUT",
        "/api/v2/tabs/3",
        **{"body": "enabled=1", "headers": FORM},
    ),
    admin_case("change_unknown_tab", "PUT", "/api/v2/tabs/999", {"name": "Nobody"}),
    admin_case("change_id_not_a_number", "PUT", "/api/v2/tabs/abc", {"name": "Nobody"}),
    admin_case("change_id_zero", "PUT", "/api/v2/tabs/0", {"name": "Nobody"}),
    admin_case("change_name_taken", "PUT", "/api/v2/tabs/3", {"name": "settings"}),
    admin_case("change_name_too_long", "PUT", "/api/v2/tabs/3", {"name": "x" * 51}),
    # Changing: what passes.
    admin_case("change_unknown_fields_only", "PUT", "/api/v2/tabs/3", {"nothing": 1}),
    admin_case("change_json_list", "PUT", "/api/v2/tabs/3", ["Plex"]),
    admin_case("change_name_to_the_same", "PUT", "/api/v2/tabs/3", {"name": "Plex"}),
    admin_case("change_name_to_another_letter_case", "PUT", "/api/v2/tabs/3", {"name": "PLEX"}),
    admin_case(
        "change_texts",
        "PUT",
        "/api/v2/tabs/3",
        {"name": " Plex 2 ", "image": " fontawesome::play ", "url": "https://plex2.example.com/", "url_local": ""},
    ),
    admin_case("tab_with_changed_texts", "GET", "/api/v2/tabs/3"),
    # The levels of the tab "Full" are 4 (least access) and 1 (most access).
    admin_case("change_least_access_past_most_access", "PUT", "/api/v2/tabs/8", {"group_id": 0}),
    admin_case("change_most_access_past_least_access", "PUT", "/api/v2/tabs/8", {"group_id_max": 5}),
    admin_case("change_both_levels_swapped", "PUT", "/api/v2/tabs/8", {"group_id": 1, "group_id_max": 4}),
    admin_case("change_both_levels_to_guest", "PUT", "/api/v2/tabs/8", {"group_id": 999, "group_id_max": 999}),
    admin_case("change_both_levels_as_text", "PUT", "/api/v2/tabs/8", {"group_id": "4", "group_id_max": "2"}),
    admin_case("tab_with_changed_levels", "GET", "/api/v2/tabs/8"),
    # "Full" is the default tab. Making "Plex 2" the default takes the mark from it.
    admin_case("change_default", "PUT", "/api/v2/tabs/3", {"default": 1}),
    admin_case("tabs_after_changing_the_default", "GET", "/api/v2/tabs"),
    admin_case("change_default_off", "PUT", "/api/v2/tabs/3", {"default": 0}),
    admin_case(
        "change_every_other_field",
        "PUT",
        "/api/v2/tabs/3",
        {
            "enabled": 1,
            "splash": 1,
            "ping": 1,
            "preload": 1,
            "timeout": 1,
            "timeout_ms": 120000,
            "ping_url": "plex.example.com:32400",
            "type": 2,
            "add_to_admin": 1,
            "order": 30,
            "category_id": 0,
        },
    ),
    admin_case("tab_with_every_other_field_changed", "GET", "/api/v2/tabs/3"),
    admin_case(
        "change_to_null",
        "PUT",
        "/api/v2/tabs/3",
        {"url": None, "ping_url": None, "timeout_ms": None, "splash": None},
    ),
    admin_case("tab_changed_to_null", "GET", "/api/v2/tabs/3"),
    # The order.
    ("order_as_guest", "PUT", "/api/v2/tabs", {"json": [{"id": 1, "order": 5}]}),
    admin_case("order_empty_list", "PUT", "/api/v2/tabs", []),
    admin_case("order_empty_object", "PUT", "/api/v2/tabs", {}),
    admin_case("order_entry_with_three_keys", "PUT", "/api/v2/tabs", [{"id": 1, "order": 7, "name": "x"}]),
    admin_case("order_entry_with_one_key", "PUT", "/api/v2/tabs", [{"id": 1}]),
    admin_case("order_entry_with_other_keys", "PUT", "/api/v2/tabs", [{"id": 1, "place": 8}]),
    admin_case("order_entry_with_order_zero", "PUT", "/api/v2/tabs", [{"id": 1, "order": 0}]),
    admin_case("order", "PUT", "/api/v2/tabs", [{"id": 1, "order": 20}, {"id": 2, "order": 19}]),
    admin_case("order_with_numbers_as_text", "PUT", "/api/v2/tabs", [{"id": "4", "order": "21"}]),
    admin_case("order_of_an_unknown_tab", "PUT", "/api/v2/tabs", [{"id": 999, "order": 3}]),
    admin_case("order_as_object_of_entries", "PUT", "/api/v2/tabs", {"first": {"id": 5, "order": 22}}),
    admin_case("tabs_after_ordering", "GET", "/api/v2/tabs"),
    # Deleting the tab "Settings" is allowed.
    admin_case("delete_settings_tab", "DELETE", "/api/v2/tabs/1"),
    admin_case("tabs_after_deleting", "GET", "/api/v2/tabs"),
    # The menu of the front end, with the tabs of this group. First some tabs get
    # addresses that the menu completes, and three tabs get the same place.
    admin_case("enable_the_tab_with_placeholders", "PUT", "/api/v2/tabs/7", {"enabled": 1}),
    admin_case(
        "change_to_addresses_without_a_scheme",
        "PUT",
        "/api/v2/tabs/9",
        {"url": "plex.example.com:32400/web", "url_local": "/local/{username}"},
    ),
    admin_case(
        "change_to_an_address_with_a_password",
        "PUT",
        "/api/v2/tabs/10",
        {"url": "https://user:secret@switches.example.com/x?y=1#z", "url_local": "null"},
    ),
    admin_case(
        "change_the_tab_of_organizr_itself",
        "PUT",
        "/api/v2/tabs/2",
        {"enabled": 1, "url_local": "/internal/{username}"},
    ),
    admin_case("change_a_tab_for_guests", "PUT", "/api/v2/tabs/11", {"group_id": 999, "order": 7}),
    admin_case(
        "change_a_tab_for_guests_only_and_admins",
        "PUT",
        "/api/v2/tabs/12",
        {"enabled": 1, "group_id": 999, "group_id_max": 999, "add_to_admin": 1, "order": 7},
    ),
    ("launch_with_tabs_as_guest", "GET", "/api/v2/launch"),
    admin_case("launch_with_tabs", "GET", "/api/v2/launch"),
]

# The category routes. A fresh install has the category "Unsorted" (id 1, number 0). The
# categories that the cases add get the ids from 2 on, in the order of the cases.
#
# The list of categories is the list of the tab editor, so it shows the tabs too. The
# group ends with the install as it found it, because the tab group is recorded after it
# and its lists show the categories.
CATEGORIES_PATH = "/api/v2/categories"
MEDIA = {"category": "Media", "image": "fontawesome::film"}

CATEGORIES = [
    # Reading.
    ("categories_as_guest", "GET", CATEGORIES_PATH),
    admin_case("categories", "GET", CATEGORIES_PATH),
    # Adding: what is refused.
    ("add_as_guest", "POST", CATEGORIES_PATH, {"json": MEDIA}),
    admin_case("add_without_a_body", "POST", CATEGORIES_PATH),
    admin_case("add_empty_object", "POST", CATEGORIES_PATH, {}),
    admin_case("add_json_list", "POST", CATEGORIES_PATH, ["Media"]),
    admin_case("add_unknown_fields_only", "POST", CATEGORIES_PATH, {"nothing": 1}),
    admin_case("add_without_name", "POST", CATEGORIES_PATH, {"image": "i"}),
    admin_case("add_without_image", "POST", CATEGORIES_PATH, {"category": "Media"}),
    # Adding: the least a category needs (id 2), and names.
    admin_case("add", "POST", CATEGORIES_PATH, MEDIA),
    admin_case("add_name_taken", "POST", CATEGORIES_PATH, MEDIA),
    admin_case("add_name_taken_in_another_letter_case", "POST", CATEGORIES_PATH, {**MEDIA, "category": "MEDIA"}),
    # The name is checked before the image.
    admin_case("add_name_taken_without_image", "POST", CATEGORIES_PATH, {"category": "media"}),
    admin_case("add_name_too_long", "POST", CATEGORIES_PATH, {**MEDIA, "category": "x" * 51}),
    admin_case("add_name_of_50_characters", "POST", CATEGORIES_PATH, {**MEDIA, "category": "y" * 50}),
    admin_case(
        "add_with_spaces_around_name_and_image",
        "POST",
        CATEGORIES_PATH,
        {"category": "  Spaced  ", "image": "  fontawesome::star  "},
    ),
    # Adding: every field (id 5), and other ways to write a value (ids 6 to 8).
    admin_case(
        "add_with_every_field",
        "POST",
        CATEGORIES_PATH,
        {"category": "Full", "image": "fontawesome::music", "order": 9, "category_id": 20, "default": 0},
    ),
    admin_case(
        "add_with_numbers_as_text",
        "POST",
        CATEGORIES_PATH,
        {"category": "Texts", "image": "i", "order": "12", "category_id": "30", "default": "0"},
    ),
    admin_case(
        "add_with_null_fields",
        "POST",
        CATEGORIES_PATH,
        {"category": "Nulls", "image": None, "order": None, "category_id": None, "default": None},
    ),
    admin_case(
        "add_as_form",
        "POST",
        CATEGORIES_PATH,
        **{"body": "category=Form&image=fontawesome%3A%3Abook", "headers": FORM},
    ),
    admin_case("categories_after_adding", "GET", CATEGORIES_PATH),
    # A name with characters that HTML gives a meaning (id 9). It is deleted again.
    admin_case("add_with_html_in_the_name", "POST", CATEGORIES_PATH, {**MEDIA, "category": "A & <b>"}),
    admin_case("categories_with_html_in_a_name", "GET", CATEGORIES_PATH),
    ("delete_as_guest", "DELETE", CATEGORIES_PATH + "/9"),
    admin_case("delete", "DELETE", CATEGORIES_PATH + "/9"),
    admin_case("delete_again", "DELETE", CATEGORIES_PATH + "/9"),
    admin_case("delete_unknown", "DELETE", CATEGORIES_PATH + "/999"),
    admin_case("delete_id_not_a_number", "DELETE", CATEGORIES_PATH + "/abc"),
    # Changing: what is refused.
    ("change_as_guest", "PUT", CATEGORIES_PATH + "/2", {"json": {"category": "Guest"}}),
    admin_case("change_without_a_body", "PUT", CATEGORIES_PATH + "/2"),
    admin_case("change_empty_object", "PUT", CATEGORIES_PATH + "/2", {}),
    admin_case("change_as_form", "PUT", CATEGORIES_PATH + "/2", **{"body": "category=Form2", "headers": FORM}),
    admin_case("change_unknown_category", "PUT", CATEGORIES_PATH + "/999", {"category": "Nobody"}),
    admin_case("change_id_not_a_number", "PUT", CATEGORIES_PATH + "/abc", {"category": "Nobody"}),
    admin_case("change_id_zero", "PUT", CATEGORIES_PATH + "/0", {"category": "Nobody"}),
    admin_case("change_name_taken", "PUT", CATEGORIES_PATH + "/2", {"category": "unsorted"}),
    admin_case("change_name_too_long", "PUT", CATEGORIES_PATH + "/2", {"category": "x" * 51}),
    # Changing: what passes.
    admin_case("change_unknown_fields_only", "PUT", CATEGORIES_PATH + "/2", {"nothing": 1}),
    admin_case("change_json_list", "PUT", CATEGORIES_PATH + "/2", ["Media"]),
    admin_case("change_name_to_the_same", "PUT", CATEGORIES_PATH + "/2", {"category": "Media"}),
    admin_case("change_name_to_another_letter_case", "PUT", CATEGORIES_PATH + "/2", {"category": "MEDIA"}),
    admin_case(
        "change_texts",
        "PUT",
        CATEGORIES_PATH + "/2",
        {"category": " Films ", "image": " fontawesome::play "},
    ),
    # "Unsorted" is the default category. Making "Films" the default takes the mark from it.
    admin_case("change_default", "PUT", CATEGORIES_PATH + "/2", {"default": 1}),
    admin_case("categories_after_changing_the_default", "GET", CATEGORIES_PATH),
    admin_case("change_default_back", "PUT", CATEGORIES_PATH + "/1", {"default": "1"}),
    admin_case("change_order_and_number", "PUT", CATEGORIES_PATH + "/2", {"order": 40, "category_id": 7}),
    # A tab names its category by the number of the category.
    admin_case("move_a_tab_into_the_category", "PUT", "/api/v2/tabs/2", {"category_id": 7}),
    admin_case("categories_after_changing", "GET", CATEGORIES_PATH),
    admin_case("move_the_tab_back", "PUT", "/api/v2/tabs/2", {"category_id": 0}),
    # The order.
    ("order_as_guest", "PUT", CATEGORIES_PATH, {"json": [{"id": 1, "order": 5}]}),
    admin_case("order_empty_list", "PUT", CATEGORIES_PATH, []),
    admin_case("order_empty_object", "PUT", CATEGORIES_PATH, {}),
    admin_case("order_entry_with_three_keys", "PUT", CATEGORIES_PATH, [{"id": 1, "order": 7, "name": "x"}]),
    admin_case("order_entry_with_one_key", "PUT", CATEGORIES_PATH, [{"id": 1}]),
    admin_case("order_entry_with_order_zero", "PUT", CATEGORIES_PATH, [{"id": 1, "order": 0}]),
    admin_case("order", "PUT", CATEGORIES_PATH, [{"id": 1, "order": 50}, {"id": 3, "order": 49}]),
    admin_case("order_with_numbers_as_text", "PUT", CATEGORIES_PATH, [{"id": "4", "order": "51"}]),
    admin_case("order_of_an_unknown_category", "PUT", CATEGORIES_PATH, [{"id": 999, "order": 3}]),
    admin_case("order_as_object_of_entries", "PUT", CATEGORIES_PATH, {"first": {"id": 5, "order": 52}}),
    admin_case("categories_after_ordering", "GET", CATEGORIES_PATH),
    # Back to the install as the group found it.
    admin_case("clean_up_order", "PUT", CATEGORIES_PATH, [{"id": 1, "order": 1}]),
    *[admin_case(f"clean_up_delete_{number}", "DELETE", f"{CATEGORIES_PATH}/{number}") for number in range(2, 9)],
    admin_case("categories_after_cleaning_up", "GET", CATEGORIES_PATH),
]

# Requests that show which route the PHP router chooses. The Rust router must choose the
# same one. Keep in step with ROUTING_CASES in crates/api/tests/slim_routes.rs.
# The group routes. A fresh install has the groups Admin (id 1, level 0), Co-Admin (2, 1),
# Super User (3, 2), Power User (4, 3), User (5, 4, the default group) and Guest (6, 999).
# The groups that the cases add get the ids from 7 on, in the order of the cases. The last
# cases delete them again, so that the install is as the cases found it.
#
# Requests that PHP takes and the Rust application refuses are not recorded, because the
# two installs would differ from there on. `docs/deviations.md` describes them.
GROUPS_PATH = "/api/v2/groups"
EDITORS = {"group": "Editors", "image": "plugins/images/groups/editors.png"}
GROUPS = [
    # Reading.
    ("groups_as_guest", "GET", GROUPS_PATH),
    admin_case("groups", "GET", GROUPS_PATH),
    admin_case("groups_with_users", "GET", GROUPS_PATH + "?includeUsers"),
    ("group_as_guest", "GET", GROUPS_PATH + "/5"),
    admin_case("group", "GET", GROUPS_PATH + "/5"),
    admin_case("group_unknown", "GET", GROUPS_PATH + "/99"),
    admin_case("group_with_an_id_that_is_no_number", "GET", GROUPS_PATH + "/abc"),
    # Adding: what is refused.
    ("add_as_guest", "POST", GROUPS_PATH, {"json": EDITORS}),
    admin_case("add_without_a_body", "POST", GROUPS_PATH),
    admin_case("add_empty_object", "POST", GROUPS_PATH, {}),
    admin_case("add_json_list", "POST", GROUPS_PATH, ["Editors"]),
    admin_case("add_unknown_fields_only", "POST", GROUPS_PATH, {"nothing": 1}),
    admin_case("add_without_name", "POST", GROUPS_PATH, {"image": "i"}),
    admin_case("add_without_image", "POST", GROUPS_PATH, {"group": "Editors"}),
    admin_case("add_with_empty_image", "POST", GROUPS_PATH, {"group": "Editors", "image": ""}),
    admin_case("add_name_too_long", "POST", GROUPS_PATH, {**EDITORS, "group": "x" * 51}),
    admin_case("add_name_of_a_seeded_group", "POST", GROUPS_PATH, {**EDITORS, "group": "user"}),
    # Adding: id 7 with level 5, and what follows from it.
    admin_case("add", "POST", GROUPS_PATH, EDITORS),
    admin_case("add_name_taken", "POST", GROUPS_PATH, EDITORS),
    admin_case("add_name_taken_in_another_letter_case", "POST", GROUPS_PATH, {**EDITORS, "group": "EDITORS"}),
    # The level is the highest level below Guest plus one, whatever the request says (id 8).
    admin_case("add_with_a_level", "POST", GROUPS_PATH, {"group": "Numbered", "image": "i", "group_id": 50}),
    admin_case("add_name_of_50_characters", "POST", GROUPS_PATH, {"group": "y" * 50, "image": "i"}),
    admin_case("add_with_spaces_around_name_and_image", "POST", GROUPS_PATH, {"group": "  Spaced  ", "image": "  i  "}),
    admin_case("add_with_null_as_default", "POST", GROUPS_PATH, {"group": "Nulls", "image": "i", "default": None}),
    admin_case("add_with_markup_in_the_name", "POST", GROUPS_PATH, {"group": "<b>Bold</b> & co", "image": "a&b.png"}),
    admin_case("groups_after_adding", "GET", GROUPS_PATH),
    admin_case("group_added", "GET", GROUPS_PATH + "/7"),
    # A new group as the default group (id 13), and the group User as the default again.
    admin_case("add_as_default", "POST", GROUPS_PATH, {"group": "Starters", "image": "i", "default": 1}),
    admin_case("groups_after_adding_a_default", "GET", GROUPS_PATH),
    admin_case("change_default_back_to_user", "PUT", GROUPS_PATH + "/5", {"default": 1}),
    admin_case("groups_after_the_default_is_back", "GET", GROUPS_PATH),
    # Changing.
    ("change_as_guest", "PUT", GROUPS_PATH + "/7", {"json": {"group": "Writers"}}),
    admin_case("change_without_a_body", "PUT", GROUPS_PATH + "/7"),
    admin_case("change_empty_object", "PUT", GROUPS_PATH + "/7", {}),
    admin_case("change_unknown_id", "PUT", GROUPS_PATH + "/99", {"group": "Writers"}),
    admin_case("change_id_zero", "PUT", GROUPS_PATH + "/0", {"group": "Writers"}),
    admin_case("change_unknown_fields_only", "PUT", GROUPS_PATH + "/7", {"nothing": 1}),
    admin_case("change_name", "PUT", GROUPS_PATH + "/7", {"group": "Writers"}),
    admin_case("change_name_to_empty", "PUT", GROUPS_PATH + "/7", {"group": ""}),
    admin_case("change_name_taken", "PUT", GROUPS_PATH + "/7", {"group": "User"}),
    admin_case("change_name_to_its_own", "PUT", GROUPS_PATH + "/7", {"group": "Writers"}),
    admin_case("change_name_to_its_own_in_another_letter_case", "PUT", GROUPS_PATH + "/7", {"group": "WRITERS"}),
    admin_case("change_name_too_long", "PUT", GROUPS_PATH + "/7", {"group": "x" * 51}),
    admin_case("change_image", "PUT", GROUPS_PATH + "/7", {"image": "new.png"}),
    admin_case("change_image_to_empty", "PUT", GROUPS_PATH + "/7", {"image": ""}),
    admin_case("change_level", "PUT", GROUPS_PATH + "/7", {"group_id": 3}),
    admin_case("change_level_to_its_own", "PUT", GROUPS_PATH + "/7", {"group_id": 5}),
    # PHP changes the row id when the request has another one. This request has the same.
    admin_case("change_with_its_own_row_id", "PUT", GROUPS_PATH + "/7", {"id": 7, "image": "again.png"}),
    admin_case("change_name_and_image", "PUT", GROUPS_PATH + "/7", {"group": "  Authors ", "image": " both.png "}),
    admin_case("change_default_on", "PUT", GROUPS_PATH + "/7", {"default": 1}),
    admin_case("groups_after_a_new_default", "GET", GROUPS_PATH),
    admin_case("change_default_on_again", "PUT", GROUPS_PATH + "/7", {"default": True}),
    admin_case("change_default_of_admin", "PUT", GROUPS_PATH + "/1", {"default": 1}),
    admin_case("change_default_of_guest", "PUT", GROUPS_PATH + "/6", {"default": 0}),
    admin_case("change_name_of_admin", "PUT", GROUPS_PATH + "/1", {"group": "Owner"}),
    admin_case("change_name_of_admin_back", "PUT", GROUPS_PATH + "/1", {"group": "Admin"}),
    admin_case("groups_after_changing", "GET", GROUPS_PATH),
    # Deleting.
    ("delete_as_guest", "DELETE", GROUPS_PATH + "/7"),
    admin_case("delete_admin", "DELETE", GROUPS_PATH + "/1"),
    admin_case("delete_guest", "DELETE", GROUPS_PATH + "/6"),
    admin_case("delete_group_with_users", "DELETE", GROUPS_PATH + "/5"),
    # The group User is the default again, so that the new group can go.
    admin_case("change_default_back_to_user_again", "PUT", GROUPS_PATH + "/5", {"default": 1}),
    admin_case("delete", "DELETE", GROUPS_PATH + "/7"),
    admin_case("group_deleted", "GET", GROUPS_PATH + "/7"),
    admin_case("groups_after_deleting", "GET", GROUPS_PATH),
    # The level of a deleted group is free again (id 14 gets level 11 or lower).
    admin_case("add_after_deleting", "POST", GROUPS_PATH, {"group": "Late", "image": "i"}),
    admin_case("group_added_after_deleting", "GET", GROUPS_PATH + "/14"),
    # Clean-up: the other new groups go.
    *[admin_case(f"clean_up_group_{id}", "DELETE", f"{GROUPS_PATH}/{id}") for id in range(8, 15)],
    admin_case("groups_at_the_end", "GET", GROUPS_PATH),
]

# The user routes. When the cases start, the install has the admin (id 1) and the users
# that the group `register` made: ids 2 to 5, with "zoe" as id 3. The users that the cases
# add get the ids from 6 on. The last cases delete them and give "zoe" her values back.
#
# Requests that PHP takes and the Rust application refuses are not recorded, because the
# two installs would differ from there on. `docs/deviations.md` describes them.
USERS_PATH = "/api/v2/users"
ABE = {"username": "Abe", "password": "the password of abe", "email": "abe@example.com"}
ZOE_IMAGE = "https://www.gravatar.com/avatar/76cec4168e20ad317a8afb3a81ae3fce?s=100&d=mm"


def as_user(jar, name, method, path, json_body=None):
    """A case that a logged-in user sends, with the cookies of the jar."""
    options = {"jar": jar}
    if json_body is not None:
        options["json"] = json_body
    return (name, method, path, options)


USERS = [
    # Reading.
    ("users_as_guest", "GET", USERS_PATH),
    admin_case("users", "GET", USERS_PATH),
    admin_case("users_with_groups", "GET", USERS_PATH + "?includeGroups"),
    ("user_as_guest", "GET", USERS_PATH + "/3"),
    admin_case("user", "GET", USERS_PATH + "/3"),
    admin_case("user_unknown", "GET", USERS_PATH + "/99"),
    admin_case("user_with_an_id_that_is_no_number", "GET", USERS_PATH + "/abc"),
    ("log_in_zoe", "POST", "/api/v2/login", {"json": {"username": "zoe", "password": ZOE["password"]}, "jar": "zoe"}),
    as_user("zoe", "users_as_user", "GET", USERS_PATH),
    as_user("zoe", "user_as_the_user", "GET", USERS_PATH + "/3"),
    # Adding: what is refused.
    ("add_as_guest", "POST", USERS_PATH, {"json": ABE}),
    as_user("zoe", "add_as_user", "POST", USERS_PATH, ABE),
    admin_case("add_without_a_body", "POST", USERS_PATH),
    admin_case("add_without_username", "POST", USERS_PATH, {"password": "pw", "email": "new@example.com"}),
    admin_case("add_without_password", "POST", USERS_PATH, {"username": "New", "email": "new@example.com"}),
    admin_case("add_without_email", "POST", USERS_PATH, {"username": "New", "password": "pw"}),
    admin_case("add_with_empty_email", "POST", USERS_PATH, {"username": "New", "password": "pw", "email": ""}),
    admin_case("add_with_invalid_email", "POST", USERS_PATH, {"username": "New", "password": "pw", "email": "not an address"}),
    admin_case("add_name_too_long", "POST", USERS_PATH, {**ABE, "username": "x" * 51}),
    admin_case("add_email_too_long", "POST", USERS_PATH, {**ABE, "email": "x" * 39 + "@example.com"}),
    admin_case("add_password_too_long", "POST", USERS_PATH, {**ABE, "password": "x" * 201}),
    admin_case("add_name_taken", "POST", USERS_PATH, {**ABE, "username": "ZOE"}),
    admin_case("add_email_taken", "POST", USERS_PATH, {**ABE, "email": "Zoe@Example.com"}),
    # Adding: id 6, and what follows from it.
    admin_case("add", "POST", USERS_PATH, ABE),
    admin_case("add_again", "POST", USERS_PATH, ABE),
    admin_case("user_added", "GET", USERS_PATH + "/6"),
    ("log_in_abe", "POST", "/api/v2/login", {"json": {"username": "abe", "password": ABE["password"]}, "jar": "abe"}),
    # Fields that are no part of a new user are ignored (id 7).
    admin_case(
        "add_with_more_fields",
        "POST",
        USERS_PATH,
        {"username": "Bea", "password": "pw", "email": "bea@example.com", "group_id": 0, "locked": 1, "image": "bea.png"},
    ),
    admin_case("user_added_with_more_fields", "GET", USERS_PATH + "/7"),
    admin_case("add_password_of_200_characters", "POST", USERS_PATH, {"username": "Cy", "password": "y" * 200, "email": "cy@example.com"}),
    admin_case("users_after_adding", "GET", USERS_PATH),
    # Changing: who may change whom.
    ("change_as_guest", "PUT", USERS_PATH + "/6", {"json": {"image": "x.png"}}),
    as_user("zoe", "change_another_user_as_user", "PUT", USERS_PATH + "/6", {"image": "x.png"}),
    as_user("zoe", "change_own_image", "PUT", USERS_PATH + "/3", {"image": "  zoe.png "}),
    as_user("zoe", "change_own_group", "PUT", USERS_PATH + "/3", {"group_id": 1}),
    as_user("zoe", "change_own_register_date", "PUT", USERS_PATH + "/3", {"register_date": "2020-01-01 00:00:00"}),
    as_user("zoe", "change_own_name_to_empty", "PUT", USERS_PATH + "/3", {"username": ""}),
    as_user("zoe", "change_own_password_to_empty", "PUT", USERS_PATH + "/3", {"password": ""}),
    as_user("zoe", "change_own_password", "PUT", USERS_PATH + "/3", {"password": "another password of zoe"}),
    ("log_in_zoe_with_the_old_password", "POST", "/api/v2/login", {"json": {"username": "zoe", "password": ZOE["password"]}}),
    ("log_in_zoe_with_the_new_password", "POST", "/api/v2/login", {"json": {"username": "zoe", "password": "another password of zoe"}}),
    admin_case("user_after_own_changes", "GET", USERS_PATH + "/3"),
    # Changing: the fields, with the API key.
    admin_case("change_id_zero", "PUT", USERS_PATH + "/0", {"image": "x.png"}),
    admin_case("change_unknown", "PUT", USERS_PATH + "/99", {"image": "x.png"}),
    admin_case("change_unknown_fields_only", "PUT", USERS_PATH + "/6", {"nothing": 1}),
    admin_case("change_name", "PUT", USERS_PATH + "/6", {"username": "  Abel "}),
    admin_case("change_name_taken", "PUT", USERS_PATH + "/6", {"username": "ZOE"}),
    admin_case("change_name_to_the_email_of_another_user", "PUT", USERS_PATH + "/6", {"username": "zoe@example.com"}),
    admin_case("change_name_to_its_own_in_another_letter_case", "PUT", USERS_PATH + "/6", {"username": "ABEL"}),
    admin_case("change_name_too_long", "PUT", USERS_PATH + "/6", {"username": "x" * 51}),
    admin_case("change_email", "PUT", USERS_PATH + "/6", {"email": "Abel@Example.com"}),
    admin_case("change_email_to_empty", "PUT", USERS_PATH + "/6", {"email": ""}),
    admin_case("change_email_invalid", "PUT", USERS_PATH + "/6", {"email": "not an address"}),
    admin_case("change_email_taken", "PUT", USERS_PATH + "/6", {"email": "ZOE@example.com"}),
    admin_case("change_email_too_long", "PUT", USERS_PATH + "/6", {"email": "x" * 39 + "@example.com"}),
    admin_case("change_group", "PUT", USERS_PATH + "/6", {"group_id": 2}),
    admin_case("change_group_unknown", "PUT", USERS_PATH + "/6", {"group_id": 77}),
    admin_case("change_group_as_text", "PUT", USERS_PATH + "/6", {"group_id": "1"}),
    admin_case("change_password", "PUT", USERS_PATH + "/6", {"password": "a new password of abe"}),
    ("log_in_abe_with_the_new_password", "POST", "/api/v2/login", {"json": {"username": "abel", "password": "a new password of abe"}}),
    admin_case("change_locked", "PUT", USERS_PATH + "/6", {"locked": 1}),
    admin_case("user_locked", "GET", USERS_PATH + "/6"),
    admin_case("change_unlocked", "PUT", USERS_PATH + "/6", {"locked": 0}),
    admin_case("change_register_date", "PUT", USERS_PATH + "/6", {"register_date": "2020-01-01 00:00:00"}),
    admin_case("change_several_fields", "PUT", USERS_PATH + "/7", {"username": "Bea B", "email": "bea.b@example.com", "image": "<b>.png"}),
    admin_case("users_after_changing", "GET", USERS_PATH),
    # A Co-Admin (id 6 has the level 1 now) and the Admin.
    as_user("abe", "co_admin_changes_the_admin", "PUT", USERS_PATH + "/1", {"image": "x.png"}),
    as_user("abe", "co_admin_changes_a_user", "PUT", USERS_PATH + "/7", {"group_id": 3}),
    as_user("abe", "co_admin_lists_the_users", "GET", USERS_PATH),
    # Deleting.
    ("delete_as_guest", "DELETE", USERS_PATH + "/7"),
    as_user("zoe", "delete_as_user", "DELETE", USERS_PATH + "/7"),
    as_user("abe", "delete_yourself", "DELETE", USERS_PATH + "/6"),
    admin_case("delete_unknown", "DELETE", USERS_PATH + "/99"),
    admin_case("delete_with_an_id_that_is_no_number", "DELETE", USERS_PATH + "/abc"),
    as_user("abe", "delete_as_co_admin", "DELETE", USERS_PATH + "/7"),
    admin_case("delete_again", "DELETE", USERS_PATH + "/7"),
    admin_case("user_deleted", "GET", USERS_PATH + "/7"),
    # A deleted user is logged out everywhere.
    admin_case("delete_a_user_with_a_session", "DELETE", USERS_PATH + "/6"),
    as_user("abe", "users_as_the_deleted_user", "GET", USERS_PATH),
    # Clean-up: the other new users go, and "zoe" has her values again.
    *[admin_case(f"clean_up_user_{id}", "DELETE", f"{USERS_PATH}/{id}") for id in range(8, 10)],
    admin_case("clean_up_zoe", "PUT", USERS_PATH + "/3", {"image": ZOE_IMAGE, "password": ZOE["password"]}),
    admin_case("users_at_the_end", "GET", USERS_PATH),
]

# The configuration routes. The cases change settings and give each one its old value
# back, so that the install is as the cases found it.
#
# Requests that PHP takes and the Rust application refuses are not recorded: a key that is
# no setting, and a value that is not of the kind of its setting.
CONFIG_PATH = "/api/v2/config"
CONFIG = [
    # Reading everything.
    ("config_as_guest", "GET", CONFIG_PATH),
    admin_case("config", "GET", CONFIG_PATH),
    # Reading one setting.
    ("item_as_guest", "GET", CONFIG_PATH + "/title"),
    admin_case("item_text", "GET", CONFIG_PATH + "/title"),
    admin_case("item_number_as_text", "GET", CONFIG_PATH + "/rememberMeDays"),
    admin_case("item_switch_that_is_off", "GET", CONFIG_PATH + "/lockScreen"),
    admin_case("item_switch_that_is_on", "GET", CONFIG_PATH + "/rememberMe"),
    admin_case("item_empty_text", "GET", CONFIG_PATH + "/customCss"),
    admin_case("item_list", "GET", CONFIG_PATH + "/ignoredNewsIds"),
    admin_case("item_number", "GET", CONFIG_PATH + "/maxSocksDebugSize"),
    admin_case("item_unknown", "GET", CONFIG_PATH + "/noSuchSetting"),
    admin_case("item_in_another_letter_case", "GET", CONFIG_PATH + "/TITLE"),
    admin_case("item_with_one_more_part", "GET", CONFIG_PATH + "/title/more"),
    admin_case("item_with_a_trailing_slash", "GET", CONFIG_PATH + "/title/"),
    # Searching.
    ("search_as_guest", "GET", CONFIG_PATH + "/search/remember"),
    admin_case("search", "GET", CONFIG_PATH + "/search/remember"),
    admin_case("search_in_another_letter_case", "GET", CONFIG_PATH + "/search/REMEMBERME"),
    admin_case("search_with_the_word_in_capitals", "GET", CONFIG_PATH + "/SEARCH/rememberMeDays"),
    admin_case("search_without_a_match", "GET", CONFIG_PATH + "/search/zzzz"),
    admin_case("search_for_a_part_with_a_dash", "GET", CONFIG_PATH + "/search/INVITES-komga"),
    # Changing.
    ("change_as_guest", "PUT", CONFIG_PATH, {"json": {"title": "Home"}}),
    admin_case("change_empty_object", "PUT", CONFIG_PATH, {}),
    admin_case("change_text", "PUT", CONFIG_PATH, {"title": "Home"}, settle=True),
    admin_case("item_after_changing_text", "GET", CONFIG_PATH + "/title"),
    admin_case("change_switch", "PUT", CONFIG_PATH, {"lockScreen": True}, settle=True),
    admin_case("item_after_changing_a_switch", "GET", CONFIG_PATH + "/lockScreen"),
    admin_case("change_switch_as_text", "PUT", CONFIG_PATH, {"lockScreen": "false", "hideRegistration": "true"}, settle=True),
    admin_case("search_after_changing_switches_as_text", "GET", CONFIG_PATH + "/search/hideRegistration"),
    admin_case("change_number_as_text", "PUT", CONFIG_PATH, {"rememberMeDays": "30"}, settle=True),
    admin_case("item_after_changing_a_number", "GET", CONFIG_PATH + "/rememberMeDays"),
    admin_case("change_text_to_null", "PUT", CONFIG_PATH, {"description": None}, settle=True),
    admin_case("item_after_changing_text_to_null", "GET", CONFIG_PATH + "/description"),
    admin_case("change_with_a_form_key", "PUT", CONFIG_PATH, {"formKey": "anything", "title": "Home again"}, settle=True),
    admin_case("search_after_a_form_key", "GET", CONFIG_PATH + "/search/formkey"),
    admin_case("change_several", "PUT", CONFIG_PATH, {"title": "Third", "loginAttempts": "9", "useLogo": True}, settle=True),
    admin_case("search_after_changing_several", "GET", CONFIG_PATH + "/search/logo"),
    # Clean-up: every changed setting gets the value of a new install.
    admin_case(
        "clean_up",
        "PUT",
        CONFIG_PATH,
        {
            "title": "Organizr V2",
            "lockScreen": False,
            "hideRegistration": False,
            "rememberMeDays": "7",
            "description": "Organizr - Accept no others",
            "loginAttempts": "5",
            "useLogo": False,
        },
        settle=True,
    ),
    admin_case("search_at_the_end", "GET", CONFIG_PATH + "/search/title"),
]

# The picture routes. The cases upload one picture and delete it again, so that the
# install is as the cases found it.
#
# The uploaded file itself is not asked for: the web server of the reference sends it,
# not the PHP application.
IMAGE_PATH = "/api/v2/image"
BOUNDARY = "parity-boundary-0123456789"
MULTIPART = {"Content-Type": f"multipart/form-data; boundary={BOUNDARY}"}
# The smallest picture that PHP takes for a GIF. All its bytes are plain text characters,
# so a case file can hold it.
TINY_GIF = "GIF89a\x01\x00\x01\x00\x00\x00\x00;"


def upload(name, content_type, content, field="file"):
    """The body of a request that uploads one file."""
    return (
        f"--{BOUNDARY}\r\n"
        f'Content-Disposition: form-data; name="{field}"; filename="{name}"\r\n'
        f"Content-Type: {content_type}\r\n\r\n"
        f"{content}\r\n"
        f"--{BOUNDARY}--\r\n"
    )


def upload_case(name, body, **options):
    """A case that uploads a file with the API key."""
    return admin_case(name, "POST", IMAGE_PATH, body=body, headers=dict(MULTIPART), **options)


IMAGES = [
    # Reading.
    ("images_as_guest", "GET", IMAGE_PATH),
    admin_case("images", "GET", IMAGE_PATH),
    ("select_as_guest", "GET", IMAGE_PATH + "/select"),
    admin_case("select", "GET", IMAGE_PATH + "/select"),
    admin_case("select_with_a_search", "GET", IMAGE_PATH + "/select?search=PLEX"),
    admin_case("select_second_page", "GET", IMAGE_PATH + "/select?page=2&limit=5"),
    admin_case("select_last_page", "GET", IMAGE_PATH + "/select?search=sonarr&page=1&limit=50"),
    admin_case("select_page_after_the_last", "GET", IMAGE_PATH + "/select?page=99"),
    admin_case("select_without_a_match", "GET", IMAGE_PATH + "/select?search=zzzz"),
    # Uploading: what is refused.
    ("upload_as_guest", "POST", IMAGE_PATH, {"body": upload("parity-logo.gif", "image/gif", TINY_GIF), "headers": dict(MULTIPART)}),
    admin_case("upload_without_a_file", "POST", IMAGE_PATH),
    upload_case("upload_in_another_field", upload("parity-logo.gif", "image/gif", TINY_GIF, field="picture")),
    upload_case("upload_a_text_file", upload("note.txt", "text/plain", "just text")),
    upload_case("upload_with_an_ending_that_is_not_allowed", upload("parity-logo.bmp", "image/gif", TINY_GIF)),
    # Uploading a picture, and what follows from it.
    upload_case("upload", upload("parity-logo.gif", "image/gif", TINY_GIF)),
    admin_case("images_after_uploading", "GET", IMAGE_PATH),
    admin_case("select_the_uploaded_picture", "GET", IMAGE_PATH + "/select?search=parity"),
    upload_case("upload_again", upload("parity-logo.gif", "image/gif", TINY_GIF)),
    # Deleting.
    ("delete_as_guest", "DELETE", IMAGE_PATH + "/parity-logo.gif"),
    admin_case("delete_unknown", "DELETE", IMAGE_PATH + "/no-such-picture.gif"),
    admin_case("delete_with_an_ending_that_is_not_allowed", "DELETE", IMAGE_PATH + "/note.txt"),
    admin_case("delete_a_picture_of_organizr", "DELETE", IMAGE_PATH + "/plex.png"),
    admin_case("delete", "DELETE", IMAGE_PATH + "/parity-logo.gif"),
    admin_case("delete_again", "DELETE", IMAGE_PATH + "/parity-logo.gif"),
    admin_case("images_at_the_end", "GET", IMAGE_PATH),
]

# The ping routes. The cases name only targets that do not answer: a port on which
# nothing listens, and a host that does not exist. The time of an answer differs from run
# to run, so a case with a target that answers cannot be compared.
PING_PATH = "/api/v2/ping"
CLOSED_PORT = "127.0.0.1:1"
NO_SUCH_HOST = "no-such-host.invalid:80"
PINGS = [
    ("ping_many_as_guest", "POST", PING_PATH, {"json": {"list": [CLOSED_PORT]}}),
    ("ping_one_as_guest", "GET", f"{PING_PATH}/{CLOSED_PORT}"),
    admin_case("ping_without_a_body", "POST", PING_PATH),
    admin_case("ping_empty_object", "POST", PING_PATH, {}),
    admin_case("ping_empty_list", "POST", PING_PATH, {"list": []}),
    admin_case("ping_empty_text", "POST", PING_PATH, {"list": ""}),
    admin_case("ping_the_text_zero", "POST", PING_PATH, {"list": "0"}),
    admin_case("ping_a_list", "POST", PING_PATH, {"list": [CLOSED_PORT, NO_SUCH_HOST]}),
    admin_case("ping_a_list_of_one", "POST", PING_PATH, {"list": [CLOSED_PORT]}),
    admin_case("ping_one_as_text", "POST", PING_PATH, {"list": CLOSED_PORT}),
    admin_case("ping_one", "GET", f"{PING_PATH}/{CLOSED_PORT}"),
    admin_case("ping_one_that_is_zero", "GET", f"{PING_PATH}/0"),
    admin_case("ping_a_whole_address", "GET", f"{PING_PATH}/http:%2F%2F{CLOSED_PORT}%2Fweb"),
]

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


# The secret of the second factor that the reference made last. A case can send it
# back, and codes made from it:
#   {2fa_secret}        the secret
#   {2fa_code}          the code of this moment
#   {2fa_code:-5}       the code of five steps of 30 seconds ago; `+5` is in the future
#   {2fa_wrong_code}    six digits that are no code of the steps around this moment
two_factor_secret = None
TWO_FACTOR_PLACEHOLDER = re.compile(r"\{2fa_(secret|code|wrong_code)(?::([+-]\d+))?\}")
TWO_FACTOR_STEP = 30


def two_factor_code(secret, time_in_seconds):
    """The code of RFC 6238 that authenticator apps show: SHA-1, six digits."""
    key = base64.b32decode(secret.upper() + "=" * (-len(secret) % 8))
    counter = struct.pack(">Q", int(time_in_seconds) // TWO_FACTOR_STEP)
    digest = hmac.new(key, counter, hashlib.sha1).digest()
    offset = digest[-1] & 0x0F
    number = struct.unpack(">I", digest[offset : offset + 4])[0] & 0x7FFFFFFF
    return f"{number % 1_000_000:06d}"


def fill_in_two_factor(text):
    def value_of(match):
        if two_factor_secret is None:
            sys.exit("No answer has held the secret of a second factor yet.")
        now = time.time()
        # Not in the last seconds of a step: the reference must see the same step.
        if now % TWO_FACTOR_STEP > TWO_FACTOR_STEP - 3:
            time.sleep(TWO_FACTOR_STEP - now % TWO_FACTOR_STEP + 0.1)
            now = time.time()
        kind, steps = match.group(1), int(match.group(2) or 0)
        if kind == "secret":
            return two_factor_secret
        if kind == "code":
            return two_factor_code(two_factor_secret, now + steps * TWO_FACTOR_STEP)
        near = {
            two_factor_code(two_factor_secret, now + step * TWO_FACTOR_STEP)
            for step in range(-8, 9)
        }
        return next(code for code in (f"{n:06d}" for n in range(1_000_000)) if code not in near)

    return TWO_FACTOR_PLACEHOLDER.sub(value_of, text)


def fill_in(text):
    """Replace each `{session_token:<jar>}` with the session token of that jar, and each
    placeholder of the second factor with its value."""
    text = fill_in_two_factor(text)

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
        payload = fill_in(json.dumps(json_body)).encode("utf-8")
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

    # An answer that holds the secret of a second factor: later cases use the secret.
    # The secret is also inside an address of the answer, so every copy is replaced.
    try:
        made = json.loads(text)["response"]["data"]["secret"]
    except (ValueError, KeyError, TypeError):
        made = None
    if isinstance(made, str) and made:
        global two_factor_secret
        two_factor_secret = made
        text = text.replace(f'"secret": "{made}"', '"secret": "<kept>"').replace(made, "<secret>")
        text = text.replace('"secret": "<kept>"', f'"secret": "{made}"')

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
        settle = options.pop("settle", False)
        cases.append({"name": name, **send(method, path, **options)})
        if settle:
            # PHP keeps compiled files for some seconds, the configuration file among
            # them. Until then it answers with the settings from before the change, and a
            # second change would write the old value of the first one back.
            time.sleep(CODE_CACHE_SECONDS)
    return cases


def prepare_registration():
    """Give the reference a registration password that works. Nothing of this is recorded.

    The set-up wizard of PHP stores the registration password as it is, but the
    registration reads it as encrypted text. So after a set-up nobody can register, until
    an admin saves the password on the settings page. This does what that page does.
    """
    JARS.clear()
    send("POST", "/api/v2/login", json_body=ADMIN, jar="admin")
    saved = send("PUT", "/api/v2/config", json_body={"registrationPassword": REGISTRATION}, jar="admin")
    if envelope(saved).get("result") != "success":
        sys.exit(f"The registration password was not saved:\n{saved['response']['body']}")

    # PHP keeps compiled files for up to two seconds, the configuration file among them.
    # Wait until a registration with the new password gets past the password check. The
    # address is not valid, so this request creates nothing.
    probe = {**ZOE, "email": "not-an-address"}
    for _ in range(40):
        answer = send("POST", "/api/v2/users/register", json_body=probe)
        if answer["response"]["status"] == 409:
            return
        time.sleep(0.25)
    sys.exit("The reference did not start to use the new registration password.")


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
    prepare_registration()
    record("register", named(REGISTER), php_version)

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

    # Last, because these groups change the tabs and categories that other groups see.
    # The category group and the group for groups leave the install as they found it,
    # for the tab group.
    record("categories", named(CATEGORIES), php_version)
    record("groups", named(GROUPS), php_version)
    record("users", named(USERS), php_version)
    record("two_factor", named(two_factor_cases()), php_version)
    record("images", named(IMAGES), php_version)
    record("ping", named(PINGS), php_version)
    record("tabs", named(TABS), php_version)
    # After the tab group, whose launch cases show the settings.
    record("config", named(CONFIG), php_version)
    # Last of all: it switches the mail plugin on, which the other groups would show.
    record("mail", named(mail_cases()), php_version)
    record_mails()


if __name__ == "__main__":
    main()
