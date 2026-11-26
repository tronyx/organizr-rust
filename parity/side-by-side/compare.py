"""Compares the GET answers of PHP Organizr and of this version on the same data.

Each route is asked as a guest and with the API key of the old install. The script prints
the status of each answer and the JSON paths that differ, that only one side has, or whose
type differs. It never prints a value, so no secret of the copied install reaches the
output. Only GET requests are sent, so nothing changes on either side.

    python -I compare.py [route ...]

PHP_ADDRESS and RUST_ADDRESS name the two servers (host:port). The API key is read from
/oldcfg/config.php, the configuration file of the old install.
"""

import http.client
import json
import os
import re
import sys

HOST = "organizr.side-by-side.test"
CONFIG = "/oldcfg/config.php"

ROUTES = [
    "/api/v2/status",
    "/api/v2/launch",
    "/api/v2/tabs",
    "/api/v2/categories",
    "/api/v2/groups",
    "/api/v2/users",
    "/api/v2/config",
    "/api/v2/plugins",
    "/api/v2/plugins/enabled",
    "/api/v2/homepage/items",
    "/api/v2/image",
    "/api/v2/icon",
    "/api/v2/page",
    "/api/v2/backup",
    "/api/v2/log",
    "/api/v2/themes",
    "/api/v2/token",
    "/api/v2/plugins/bookmark/page",
    "/api/v2/plugins/invites",
    "/api/v2/plugins/chat/message",
    "/api/v2/auth?group=0",
    "/api/v2/auth?group=1",
    "/api/v2/auth?group=4",
    "/api/v2/auth?group=999",
    "/api/v2/auth-1",
    "/api/v2/does-not-exist",
]

# The lists show their first entries only, so a long list doesn't flood the output.
LIST_ENTRIES = 200
SHOWN = 12


def address(name, default):
    host, _, port = os.environ.get(name, default).rpartition(":")
    return host, int(port)


def api_key():
    try:
        text = open(CONFIG, encoding="utf-8", errors="replace").read()
    except OSError:
        return ""
    found = re.search(r"'organizrAPI'\s*=>\s*'([^']*)'", text)
    return found.group(1) if found else ""


def get(server, path, key):
    connection = http.client.HTTPConnection(*server, timeout=20)
    headers = {"Host": HOST, "Accept": "application/json"}
    if key:
        headers["Token"] = key
    try:
        connection.request("GET", path, headers=headers)
        answer = connection.getresponse()
        return answer.status, answer.getheader("Content-Type", ""), answer.read()
    except (OSError, http.client.HTTPException) as error:
        return "failed", type(error).__name__, b""
    finally:
        connection.close()


def shapes(value, path="$"):
    """Each JSON path with a description of its value: its type, and for a leaf a
    fingerprint to compare. The fingerprint is never printed."""
    found = {}
    if isinstance(value, dict):
        found[path] = ("object", None)
        for key, inner in value.items():
            found.update(shapes(inner, f"{path}.{key}"))
    elif isinstance(value, list):
        found[path] = (f"list of {len(value)}", None)
        for index, inner in enumerate(value[:LIST_ENTRIES]):
            found.update(shapes(inner, f"{path}[{index}]"))
    else:
        found[path] = (type(value).__name__, json.dumps(value, sort_keys=True))
    return found


def compare(route, php, rust, key, caller):
    before = get(php, route, key)
    after = get(rust, route, key)
    line = f"{caller:6} {route:40} php={before[0]} rust={after[0]}"
    try:
        old, new = json.loads(before[2]), json.loads(after[2])
    except ValueError:
        print(
            line,
            f"| not JSON: php {before[1][:30]!r} {len(before[2])} bytes,"
            f" rust {after[1][:30]!r} {len(after[2])} bytes",
        )
        return
    old, new = shapes(old), shapes(new)
    both = set(old) & set(new)
    differ = sorted(path for path in both if old[path] != new[path])
    php_only = sorted(set(old) - set(new))
    rust_only = sorted(set(new) - set(old))
    print(
        line,
        f"| same {len(both) - len(differ)}, differ {len(differ)},"
        f" php only {len(php_only)}, rust only {len(rust_only)}",
    )
    for path in differ[:SHOWN]:
        print("    differ   ", path, old[path][0], "->", new[path][0])
    for path in php_only[:SHOWN]:
        print("    php only ", path)
    for path in rust_only[:SHOWN]:
        print("    rust only", path)


def main():
    php = address("PHP_ADDRESS", "php:80")
    rust = address("RUST_ADDRESS", "rust:8080")
    key = api_key()
    if not key:
        print(f"No API key found in {CONFIG}; only the guest answers are compared.")
    for route in sys.argv[1:] or ROUTES:
        compare(route, php, rust, None, "guest")
        if key:
            compare(route, php, rust, key, "apikey")


if __name__ == "__main__":
    main()
