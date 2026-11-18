"""Record what the PHP application answers, as the ground truth for the parity tests.

Run from the repository root, against a fresh reference container:

    docker compose -f parity/oracle/compose.yml up --build --detach
    python parity/recorder/record.py
    docker compose -f parity/oracle/compose.yml down --volumes

The recorder first records a few answers of the empty install. Then it runs the set-up
wizard with made-up values and records the other groups. Each group becomes one file in
parity/cases/. Recording again must give the same files, so values that change on every
request (the device id, the caller address) are replaced by placeholders.

Only the Python standard library is used. No data of a real install is involved.
"""

import http.client
import json
import os
import re
import sys
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


def send(method, path, body=None):
    """Send one request to the reference and return the parts of the answer that matter."""
    target = urlsplit(ORACLE_URL)
    connection = http.client.HTTPConnection(target.hostname, target.port or 80, timeout=60)
    headers = {"Host": HOST}
    payload = None
    if body is not None:
        payload = json.dumps(body).encode("utf-8")
        headers["Content-Type"] = "application/json"
    try:
        connection.request(method, path, body=payload, headers=headers)
        response = connection.getresponse()
        text = response.read().decode("utf-8")
        recorded_headers = {}
        cookies = []
        for name, value in response.getheaders():
            lower = name.lower()
            if lower in RECORDED_HEADERS:
                recorded_headers[lower] = value
            elif lower == "set-cookie":
                name = value.split("=", 1)[0]
                cookies.append(SESSION_COOKIE.sub("organizr_token_<uuid>", name))
        status = response.status
    finally:
        connection.close()

    redacted = []
    for pattern, replacement in REDACTIONS:
        text, count = pattern.subn(replacement, text)
        if count:
            redacted.append(replacement.split("<")[1].split(">")[0])

    request = {"method": method, "path": path}
    if body is not None:
        request["json"] = body
    answer = {"status": status, "headers": recorded_headers, "cookies_set": cookies, "body": text}
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
    return [{"name": name, **send(method, path)} for name, method, path in definitions]


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
    wizard = send("POST", "/api/v2/wizard", body=SEED)
    if envelope(wizard).get("result") != "success":
        sys.exit(f"The set-up wizard failed:\n{wizard['response']['body']}")
    fresh.append({"name": "wizard", **wizard})
    record("fresh_install", fresh, php_version)

    record("basics", named(BASICS), php_version)

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
