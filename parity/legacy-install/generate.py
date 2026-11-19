"""Make the made-up PHP install that the tests of the importer read.

The importer moves an install of PHP Organizr into the Rust application. Its tests need
the two things such an install holds: the configuration file and the SQLite database,
as the real PHP application writes them. This script sets up the reference container,
fills it through the API, and stores both under `crates/import/tests/fixtures/legacy/`:

  config.php      the configuration file, as PHP wrote it
  organizr.sql    the database, as SQL text

Every value is made up and protects nothing. Start a fresh reference, run the script
from the root of the repository, and throw the reference away:

  docker compose -f parity/oracle/compose.yml up --build --detach
  PYTHONDONTWRITEBYTECODE=1 python -I parity/legacy-install/generate.py
  docker compose -f parity/oracle/compose.yml down --volumes

Rows that only other parts of PHP Organizr write (a login with a second factor, a login
through Plex, invites, bookmarks, chat) are added to the database file directly, in the
form in which those parts store them.
"""

import pathlib
import sqlite3
import subprocess
import sys
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "parity" / "recorder"))

import record  # noqa: E402  (the path above must be set first)

COMPOSE = ["docker", "compose", "-f", str(ROOT / "parity" / "oracle" / "compose.yml")]
TARGET = ROOT / "crates" / "import" / "tests" / "fixtures" / "legacy"
CONFIG_IN_CONTAINER = "/var/www/html/data/config/config.php"
DATABASE_IN_CONTAINER = record.SEED["dbPath"] + record.SEED["dbName"] + ".db"

# What an Admin does on the settings pages. PHP encrypts each value whose key holds the
# word "password". `MB-URL` is a key of a plugin that is not part of Organizr.
CONFIG = {
    "title": "Legacy Install",
    "unsortedTabs": "bottom",
    "localIPList": "10.0.0.0/8",
    "loginAttempts": "5",
    "plexToken": "made-up-plex-token",
    "plexURL": "http://plex.example.com:32400",
    "homepagePlexStreams": True,
    "qBittorrentPassword": "made-up torrent password",
    "PHPMAILER-smtpHostPassword": "made-up mail password",
    "registrationPassword": "let-new-users-in",
    "MB-URL": "https://mediabutler.example.com",
}

# Rows as other parts of PHP Organizr store them.
EXTRA_ROWS = """
UPDATE users SET auth_service = 'google::JBSWY3DPEHPK3PXP' WHERE username = 'robin';
UPDATE users SET auth_service = 'plex', plex_token = 'made-up-token-of-a-plex-user',
    locked = NULL, register_date = '2018-05-08T01:28:03Z' WHERE username = 'pat';
INSERT INTO invites (code, date, email, username, dateused, usedby, ip, valid, type, invitedby)
    VALUES ('MADEUP', '2026-01-02 03:04:05', 'guest@example.com', 'guest', NULL, NULL,
            '203.0.113.9', 'Yes', 'Plex', 'admin');
INSERT INTO "BOOKMARK-categories" ("order", category, category_id, "default")
    VALUES (1, 'Reading', 1, 1);
INSERT INTO "BOOKMARK-tabs" ("order", category_id, name, url, enabled, group_id, image,
                             background_color, text_color)
    VALUES (1, 1, 'News', 'https://news.example.com/', 1, 999,
            'plugins/images/tabs/news.png', '#000000', '#ffffff');
INSERT INTO chatroom (username, gravatar, uid, date, ip, message)
    VALUES ('admin', '', 'f1f17', '2026-01-02 03:04:05', '203.0.113.9', 'Hello');
"""


def ask(method, path, body=None, as_admin=True):
    """Send a request that must succeed. The API key counts as the Admin."""
    headers = record.AS_ADMIN if as_admin else None
    answer = record.send(method, path, json_body=body, headers=headers)
    status = answer["response"]["status"]
    if status != 200:
        sys.exit(f"{method} {path} answered {status}:\n{answer['response']['body']}")
    return record.envelope(answer).get("data")


def fill():
    status = record.envelope(record.send("GET", "/api/v2/status")).get("data") or {}
    if status.get("action") != "wizard":
        sys.exit("The reference is not a fresh install. Start a new one first.")
    ask("POST", "/api/v2/wizard", record.SEED, as_admin=False)

    ask(
        "POST",
        "/api/v2/groups",
        {"group": "Editors", "image": "plugins/images/groups/editors.png"},
    )
    for name, level in (("robin", 4), ("pat", 4), ("Sam", 2)):
        ask(
            "POST",
            "/api/v2/users",
            {
                "username": name,
                "email": f"{name.lower()}@example.com",
                "password": f"the password of {name.lower()}",
            },
        )
        users = ask("GET", "/api/v2/users")
        user = next(user for user in users if user["username"] == name)
        if level != 4:
            ask("PUT", f"/api/v2/users/{user['id']}", {"group_id": level})

    ask(
        "POST",
        "/api/v2/categories",
        {"category": "Media", "image": "fontawesome::film", "category_id": 7},
    )
    ask(
        "POST",
        "/api/v2/tabs",
        {
            "name": "Plex",
            "url": "https://{domain}/plex",
            "url_local": "http://10.0.0.6:32400/web",
            "image": "plugins/images/tabs/plex.png",
            "enabled": 1,
            "category_id": 7,
            "group_id": 4,
            "ping": 1,
            "ping_url": "10.0.0.6:32400",
            "preload": 1,
            "default": 1,
        },
    )
    ask(
        "POST",
        "/api/v2/tabs",
        {
            "name": "Docs",
            "url": "https://docs.example.com/",
            "image": "fontawesome::book",
            "enabled": 0,
            "type": 2,
            "group_id": 999,
            "group_id_max": 2,
            "add_to_admin": 1,
        },
    )
    ask("PUT", "/api/v2/config", CONFIG)
    # PHP keeps the compiled configuration file for a few seconds.
    time.sleep(4.5)


def copy_out(path_in_container, target):
    subprocess.run([*COMPOSE, "cp", f"oracle:{path_in_container}", str(target)], check=True)


def main():
    fill()
    TARGET.mkdir(parents=True, exist_ok=True)
    copy_out(CONFIG_IN_CONTAINER, TARGET / "config.php")

    with tempfile.TemporaryDirectory() as folder:
        database = pathlib.Path(folder) / "organizr.db"
        copy_out(DATABASE_IN_CONTAINER, database)
        connection = sqlite3.connect(database)
        connection.executescript(EXTRA_ROWS)
        connection.commit()
        dump = "\n".join(connection.iterdump()) + "\n"
        connection.close()
    (TARGET / "organizr.sql").write_text(dump, encoding="utf-8", newline="\n")
    print(f"Wrote {TARGET / 'config.php'} and {TARGET / 'organizr.sql'}")


if __name__ == "__main__":
    main()
