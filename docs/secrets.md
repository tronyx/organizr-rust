# 🔑 The key for stored secrets

[← All documentation](README.md)

Organizr encrypts the secrets it stores (API keys, tokens, passwords of your services, second-factor secrets) with one key: `secret.key`. This page tells you where that key lives, how to back it up, and what to do if it goes missing. Read it once, right after the install, and make that backup: it takes a minute and saves you a very bad day.

## 🧠 What the key does

A **secret** is a value that opens something: the Organizr API key, the tokens and passwords of the services you connect, the Plex tokens of your users, and the [second factor](second-factor.md) of each account. Organizr encrypts each secret before it stores it in the database.

- The key is 32 random bytes, written as 64 hex digits.
- **The key is not in the database.** Someone with a copy of the database but not the key can't read the secrets.
- **Without the key, nobody can read them, not even you.** Organizr then starts without them, and you have to enter each one again.

Everything else in the database is **not** encrypted: user names, tab addresses and the other settings are plain text. User passwords are stored as hashes (a value that can check a password but doesn't reveal it), so they don't need the key. A database copy without the key is therefore not "safe", it just doesn't show the secrets.

## 📍 Where the key is

Organizr reads the key at start-up, from the first of these that is set:

| Order | Source | Notes |
|---|---|---|
| 1 | `ORGANIZR_SECRET_KEY` | The key itself, as 64 hex digits. When set, Organizr reads and writes no key file |
| 2 | `ORGANIZR_SECRET_KEY_FILE` | The path of a key file |
| 3 | `secret.key` in the data folder | The default. In the container that's `/config/secret.key`, inside the `organizr-data` volume |

**On the first start**, when no variable is set and the key file doesn't exist yet, Organizr makes a new key, writes it to the key file (readable only by the user that runs Organizr, mode `0600`), and logs this warning:

```text
a new key for the stored secrets was written; keep a copy of this file in a safe place apart from your backups, because the secrets in the database cannot be read without it
```

See [Back up the key](#-back-up-the-key) for how to keep that copy.

If the variable or the file holds something that isn't a key, Organizr refuses to start and says so.

## 💾 Back up the key

The key never changes, so **one copy, made once, is enough**.

> [!IMPORTANT]
> **The backups Organizr makes don't include `secret.key`** (**Settings › System Settings › Backup**). That's on purpose: a backup with its key next to it is as open as a database without encryption. Keep your copy of the key **somewhere else** than your backup files, for example in your password manager, but somewhere you can still reach when you need to restore.

To copy the key out of a Compose install, run this in the folder with your compose file:

```bash
docker compose cp organizr:/config/secret.key ./organizr-secret.key
```

Then move `organizr-secret.key` into your password manager or another safe place, and delete the local copy. (With `docker run`, use `docker cp organizr:/config/secret.key ./organizr-secret.key`, where `organizr` is your container's name.)

| Your setup | What a full backup needs |
|---|---|
| **SQLite** (default) | The database is in the same volume as the key. A copy of the whole volume holds both, so store such a copy as carefully as the key itself |
| **PostgreSQL** | The database lives in its own container or server. A dump of PostgreSQL doesn't hold the key. Back up the Organizr data volume (or just the key) as well |

## 🆘 If the key is lost or wrong

Organizr still starts when it can't read a stored secret. It logs one line for each such secret:

```text
a stored setting is ignored problem=the stored secret `plexToken` cannot be opened with the key of this install
```

What breaks:

| What | What happens |
|---|---|
| Settings with secrets (tokens, API keys, passwords of services) | Empty until you enter them again |
| The Organizr API key | Scripts get `401` until you set a new key |
| **Accounts with a second factor, the Admin's too** | **Can't log in.** Every code is refused, because the second-factor secret can't be read. The log says: `the stored secret of the second factor of the user cannot be used` |
| Accounts without a second factor | Log in as usual. Passwords don't need the key |

### 🔧 Fix it

1. **Stop Organizr**, and **don't re-enter any secret yet.** Anything you enter now is encrypted with the wrong key, and is lost again once the right key is back.
2. **Have a copy of the right key?** Put it back (see [Put a key file back](#-put-a-key-file-back)), and start Organizr. Done: everything is readable again.
3. **The key is really gone?** Start Organizr with the new key it made, and then:
   1. Remove the second factor of every account that had one, the Admin's included, as [Lost your phone?](second-factor.md#-lost-your-phone) shows. Then each user sets it up again.
   2. Log in, and enter each secret again in the settings.
   3. Make a new API key for your scripts.
   4. **Back up the new key** this time.

### 🔎 A new key made by mistake

A wrong mount or path makes Organizr think it's a first start, and it **quietly makes a new key**. Typical causes: a typo in `ORGANIZR_SECRET_KEY_FILE`, a missing `/config` volume (easy to forget with PostgreSQL, where the database lives elsewhere), or a renamed Compose project, which means a new, empty `<project>_organizr-data` volume.

How to notice:

- The log shows `a new key for the stored secrets was written` on an install that already existed.
- Right after it, `a stored setting is ignored` lines appear, the secret fields in the settings are empty, and users with a second factor can't log in.
- With SQLite, a wrong volume also means an empty database, so you see the set-up wizard again.

If that happens, stop Organizr, fix the volume or the path so it points at the real key file, and start again. Don't enter any secret before that (see step 1 above).

### 📥 Put a key file back

With Compose, copy your saved key into the volume with a throw-away container, so that the file gets the right owner (UID 65532) and mode:

```bash
docker compose stop organizr
docker run --rm -v <project>_organizr-data:/config -v "$PWD":/backup:ro alpine:3 \
  sh -c 'cp /backup/organizr-secret.key /config/secret.key && chown 65532:65532 /config/secret.key && chmod 600 /config/secret.key'
docker compose start organizr
```

- `<project>` is the Compose project name, usually the folder name of your compose file. `docker volume ls` shows the exact volume name.
- Run it in the folder that holds `organizr-secret.key`.

## 🔐 Keep the key out of the data volume

You may prefer the key not to sit in the volume next to the database (for example, because you copy that volume around). There are two ways.

### 📄 A key file as a Docker secret (recommended)

1. Make a key file. For a **new** install, generate a key:

   ```bash
   openssl rand -hex 32 > organizr.key
   ```

   For an **existing** install, use the existing key instead: copy `secret.key` out (see [Back up the key](#-back-up-the-key)) and rename it to `organizr.key`. A new key would lock you out of every stored secret.
2. Let only the Organizr user (UID 65532) read it:

   ```bash
   sudo chown 65532:65532 organizr.key
   sudo chmod 600 organizr.key
   ```

3. Give it to Organizr as a Compose secret:

   ```yaml
   services:
     organizr:
       secrets:
         - organizr_key
       environment:
         ORGANIZR_SECRET_KEY_FILE: /run/secrets/organizr_key

   secrets:
     organizr_key:
       file: ./organizr.key
   ```

4. Start Organizr with `docker compose up -d`. If `./organizr.key` doesn't exist, Compose refuses to start the container, so a typo can't quietly make a new key. If the path inside the container is wrong instead (`ORGANIZR_SECRET_KEY_FILE`), Organizr makes a new key there; check the log for the "new key" line (see [above](#-a-new-key-made-by-mistake)).
5. Once Organizr has started and your secrets are readable, remove the old key file from the data volume, so the key no longer sits next to the database:

   ```bash
   docker run --rm -v <project>_organizr-data:/config alpine:3 rm /config/secret.key
   ```

   `<project>` is your Compose project name (`docker volume ls` shows the volume). Do the same after a move to `ORGANIZR_SECRET_KEY`.

### 🌱 The key in an environment variable

Put the 64 hex digits in `ORGANIZR_SECRET_KEY`. Organizr then reads and writes no key file. Use the same value at every start: for an existing install, the content of its `secret.key`; for a new one, the output of `openssl rand -hex 32`.

> [!WARNING]
> An environment variable is easy to leak: anyone who can run `docker inspect` on the host, read your compose file or shell history, or read `/proc` can see it. Prefer the key file. If you use the variable, keep it in an `.env` file that only you can read (`chmod 600 .env`).

> [!CAUTION]
> **Never change the key of an install that has stored secrets.** Organizr can't read secrets stored with another key, and accounts with a second factor can't log in.
