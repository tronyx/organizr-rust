# 🩺 Troubleshooting

[← All documentation](README.md)

Something not working? Find your problem below: each entry names the cause and what to do. When things are really broken (a lost password, a lost key, a bad update), jump to [Recovery](#-recovery).

Most answers start in the log. Open **Settings › System Settings › Logs**, or run this in the folder of your `compose.yaml`:

```bash
docker compose logs organizr
```

With `docker run --name organizr`, use `docker logs organizr`. [The log](log.md) tells you more.

## 🚀 Set-up and start

### 🔢 I don't know the set-up code

The code is in the container's log. Each start makes a new code, so read the newest one:

```bash
docker compose logs organizr | grep "set-up code"
```

### 🚪 The page doesn't open at all

1. Check that the container runs: `docker compose ps` shows it `Up` and, after a few seconds, `healthy`.
2. Use the address of the machine that runs Docker, with the port: `http://192.168.1.10:8080`, not `https://`. Port 8080 speaks plain HTTP.
3. Check the port mapping in `compose.yaml` (`"8080:8080"`). If another program already uses port 8080 on the host, `docker compose up` fails with `port is already allocated`. Map another host port instead, for example `"9090:8080"`, and open `http://<your server>:9090`.
4. If the container keeps restarting, see [The container won't start](#-the-container-wont-start).

### 📁 The container stops with "cannot write to the data folder"

The container runs as the user `65532`, not as root. The log says:

```text
Error: cannot write to the data folder /config. The user that runs Organizr must be able to write to it.
```

A folder of the host that you mount as `/config` must belong to that user:

```bash
sudo chown -R 65532:65532 /path/to/the/folder
```

A named Docker volume needs nothing. See [Installation](installation.md#-the-data-folder).

### 🔑 After a restore or a move, settings with passwords and keys are empty

Organizr encrypts the stored passwords, tokens and keys with the key in `secret.key`. The new container has another key. The log has a line like this one for each value:

```text
a stored setting is ignored problem=the stored secret `plexToken` cannot be opened with the key of this install
```

Put the old `secret.key` back in the data folder and start the container again. If the key is lost, see [I lost secret.key](#-i-lost-secretkey).

### 🆕 The log says a new key was written, on an install that already existed

```text
a new key for the stored secrets was written; keep a copy of this file with your backups, because the secrets in the database cannot be read without it
```

On a brand-new install, that's normal. On an existing one, it means Organizr didn't find its key: the volume isn't the one you meant (with Compose, it's `<project>_organizr-data`), or `ORGANIZR_SECRET_KEY_FILE` points to the wrong place. Stop Organizr, fix the mount or the path, and put the right `secret.key` in place before you enter any secret again.

### 🐘 With PostgreSQL, the log says that `seaql_migrations` already exists

At each start with PostgreSQL, the log can show lines like this one:

```text
INFO sqlx::postgres::notice: relation "seaql_migrations" already exists, skipping
```

They're harmless. At each start, Organizr makes sure its tables are up to date, and PostgreSQL mentions that the table that records those updates is already there. You don't need to do anything.

### 🔒 `database is locked` with SQLite

The data folder is probably on a network share (NFS, SMB). SQLite needs a local disk, and Organizr doesn't detect or refuse a share, so it starts anyway. Move the volume to a local disk, or [use PostgreSQL](choosing-a-database.md).

## 🌐 Reverse proxy and tabs

### 🔓 The tabs open http:// addresses, but the site uses HTTPS

Organizr doesn't believe your proxy's headers. Add the proxy's address to `ORGANIZR_TRUSTED_PROXIES`, and re-create the container (`docker compose up -d`). The log has this warning when the setting is missing:

```text
a request with an X-Forwarded-For header came from an address that is not a trusted proxy, so the header was ignored.
```

See [Trusted proxies](reverse-proxy.md#-organizr_trusted_proxies).

### 🪆 A tab shows Organizr inside Organizr

The tab has a path as its address, for example `/sonarr/`. The browser asks Organizr's host for that path, and the proxy has no location for it, so it sends the request to Organizr. Add a location for the application to the proxy. See [A tab with a path](reverse-proxy.md#-a-tab-with-a-path).

### 🏷 A tab points to `organizr:8080` or another inside name

nginx sends the name of the upstream unless you tell it otherwise. Add `proxy_set_header Host $host;` to Organizr's location.

### 🪟 A tab stays empty, or the browser says it refused to connect

The browser won't show the application in a frame. Two causes are common:

- The application forbids frames with `X-Frame-Options` or `frame-ancestors`.
- Organizr uses HTTPS, and the tab uses an `http://` address.

See [A tab in a frame](reverse-proxy.md#-a-tab-in-a-frame), or set the tab to open in a new window.

### 📤 Uploads fail behind the proxy

nginx accepts only 1 MB per request by default. Add `client_max_body_size 64m;` to Organizr's server block. See [nginx](reverse-proxy.md#-nginx).

## 🔐 Login and passwords

### ✉ The e-mail with a new password doesn't arrive

1. Check that the plugin **PHP Mailer** is on in **Settings › Plugins › Active** (switch it on from **Inactive**). Without it, the login page has no **Forgot your password?** link.
2. Open the **PHP Mailer** settings. In the section **Test and options**, select **Send** next to **Send Test**. The test e-mail goes to your own account's address, and the page shows the reason the SMTP server gave.
3. Look for this line in the log:

   ```text
   the SMTP server did not take the e-mail: ...
   ```

   - `Connection refused` or a timeout: Organizr can't reach the SMTP server at that address and port. Check the host name and the port. Check that the mail server runs, and that a firewall lets the container through. If the mail server is in your own network, its public address may not work from inside: use its local address.
   - A message about the login: check the SMTP user name and password.
   - A message about TLS: check the kind of encryption. Port `587` usually needs `tls`, which is STARTTLS. Port `465` needs `ssl`, which is TLS from the start.

See [E-mail](email.md).

### 🔑 I forgot my password, and no e-mail comes

Another Admin can set a new password for you in **Settings › User Management › Manage Users**. A Co-Admin can't change an Admin's password.

Without another Admin, the e-mail is the way: fix the e-mail settings first, then select **Forgot your password?**. No way in at all? See [I lost the Admin password](#-i-lost-the-admin-password).

### 🔁 My old password still works after a reset

That's intended. The new password from the e-mail works for 24 hours next to the old one. It becomes the account's password when you log in with it. So someone who asks for a new password in your name can't lock you out. See [E-mail](email.md).

## ⚙ Settings

### 🚫 Backup says that I'm not authorized

Only an Admin or a Co-Admin can use the backups, because a backup holds every user's password hash. Log in with one of these accounts. See [Backups](backups.md).

### 🎨 My custom CSS does nothing

Custom CSS written for PHP Organizr aims at ids and classes that these pages don't have, such as `#allHealthChecks` or `.white-box`. Change the rules to the names in [Custom CSS](custom-css.md).

### 📜 My custom JavaScript doesn't run

Custom JavaScript runs only when the container has `ORGANIZR_CUSTOM_JS=1`. See [Appearance](appearance.md).

## 🏠 Homepage

### 📡 An item shows an error, or no data

1. Open the item's settings and select **Test Connection**. The page shows whether the service answered, or what's wrong. See [Homepage](homepage.md).
2. Check the address of the service. Organizr asks the service from inside its container, so `localhost` is Organizr's own container, not your server. Use the service's container name, or the server's address.
3. Run `docker compose exec organizr organizr check` to ask all your services at once. See [Checking your services](checking-your-services.md).

## 🛟 Recovery

Before you change anything here, make a copy of what you have: the volume (or at least `organizr.db` and `secret.key`) and, with PostgreSQL, a `pg_dump`.

### 🧯 The container won't start

Read the last lines of the log: `docker compose logs --tail 50 organizr`. The first line that starts with `Error:` names the problem.

| The log says | What to do |
| --- | --- |
| `cannot write to the data folder` | Give the folder to the user `65532`. See [above](#-the-container-stops-with-cannot-write-to-the-data-folder) |
| `cannot open the database` | With PostgreSQL: check `ORGANIZR_DATABASE_URL`, and that the database runs. The example's `depends_on` with `condition: service_healthy` makes Organizr wait for it. See [Choosing a database](choosing-a-database.md#-use-postgresql) |
| `cannot bring the database schema up to date` | Often an older image on a database that a newer version already updated. Go back to the newer image, or [roll back](installation.md#-roll-back) with the backup from before the update |
| `the list of trusted proxies is not valid` | Fix `ORGANIZR_TRUSTED_PROXIES`: addresses and networks only, separated by commas, no host names |
| `ORGANIZR_SECRET_KEY is not a key` | The variable must hold 64 hex digits |
| `the key file ... does not hold a key` | `secret.key` is damaged. Put your copy back. See [I lost secret.key](#-i-lost-secretkey) |
| `cannot listen on ...` | `ORGANIZR_LISTEN` is wrong, or, with host networking, another program uses the port |

[Configuration](configuration.md) lists every variable.

### ⏪ An update broke something

Go back to the image you ran before, together with the backup you made before the update. [Roll back](installation.md#-roll-back) has the steps. A database that a newer version opened doesn't open with an older image, so the backup is the part you can't skip.

### 🔑 I lost the Admin password

Try these in order:

1. **Another Admin** sets a new password in **Settings › User Management › Manage Users**.
2. **E-mail**: select **Forgot your password?** on the login page. This needs the **PHP Mailer** plugin set up.
3. **By hand**: put a new password hash into the database. Organizr accepts bcrypt hashes (as PHP Organizr made them) and replaces them with its own on your next login.

   1. Make a bcrypt hash. This command asks for the new password twice:

      ```bash
      docker run --rm -it httpd:2.4-alpine htpasswd -nBC 12 admin
      ```

      It prints `admin:$2y$12$...`. The hash is everything after `admin:`.

   2. Stop Organizr: `docker compose stop organizr`.
   3. Open the database, as shown in [Edit the database by hand](#-edit-the-database-by-hand), and run:

      ```sql
      UPDATE users SET password_hash = '<hash>' WHERE username_key = '<user name in small letters>';
      ```

   4. Start Organizr: `docker compose up -d`, and log in with the new password.

A restart also clears any login lockout, because Organizr keeps the count of failed logins in memory.

### 📱 I lost my second factor

Without the authenticator app, the account can't log in, and a new password by e-mail doesn't remove the second factor. An Admin has no button for this yet. Remove it in the database:

1. Stop Organizr: `docker compose stop organizr`.
2. Open the database, as shown in [Edit the database by hand](#-edit-the-database-by-hand), and run:

   ```sql
   UPDATE users SET two_factor_kind = NULL, two_factor_secret = NULL WHERE username_key = '<user name in small letters>';
   ```

3. Start Organizr: `docker compose up -d`. Log in with the password only, then set up the second factor again in your account. See [Second factor](second-factor.md).

### 🗝 I lost secret.key

Without the key, Organizr can't read anything it stored encrypted. It starts anyway, makes a new key, and logs a line for each value it can't read.

| What | What happens | What to do |
| --- | --- | --- |
| Passwords of users | Unaffected: they're hashes, not encrypted | Nothing |
| Accounts with a second factor | Can't log in: the secret was encrypted with the lost key | Remove the second factor, as in [I lost my second factor](#-i-lost-my-second-factor), and set it up again |
| Tokens and passwords of your services (Plex, Sonarr, SMTP and so on) | Empty | Enter each one again in the settings |
| Organizr's API key | Empty: scripts get `401` | Make a new one in **Settings › System Settings › General** (**Generate New API Key**) |

First, look hard for a copy: an old backup of the volume, your password manager, the folder of another install. If you find it, put it back:

1. Stop Organizr: `docker compose stop organizr`.
2. In the folder that holds the copy (`secret.key`), copy it into the volume, owned by Organizr's user:

   ```bash
   docker run --rm -v <project>_organizr-data:/config -v "$PWD/secret.key":/in/secret.key:ro alpine sh -c 'cp /in/secret.key /config/secret.key && chown 65532:65532 /config/secret.key && chmod 600 /config/secret.key'
   ```

3. Start Organizr: `docker compose up -d`.

See [Secrets](secrets.md).

### 🧰 Edit the database by hand

Stop Organizr first, so nothing else writes to the database.

**With SQLite**, open a throw-away container on the volume (with Compose, the volume is `<project>_organizr-data`; `docker volume ls` lists the names):

```bash
docker run --rm -it -v <project>_organizr-data:/config alpine sh
```

Then, inside that container:

```sh
cp /config/organizr.db /config/organizr.db.before-edit
apk add --no-cache sqlite
sqlite3 /config/organizr.db
```

Type the SQL from the steps above, then `.quit`. Before you leave, give the files back to Organizr's user, and close the container:

```sh
chown 65532:65532 /config/organizr.db*
exit
```

**With PostgreSQL**, open `psql` in the database container, type the SQL, then `\q`:

```bash
docker compose exec database psql --username organizr --dbname organizr
```

## 💬 Ask for help

If this page doesn't help, [open an issue](https://github.com/tronyx/organizr-rust/issues). Add:

- the version: `docker compose exec organizr organizr --version`;
- the database engine: SQLite or PostgreSQL;
- the log lines around the problem, without passwords, keys and tokens.

> [!IMPORTANT]
> Found a security problem? Don't open a public issue. Follow the [security policy](../SECURITY.md) instead.
