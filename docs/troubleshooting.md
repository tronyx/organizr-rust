# Troubleshooting

Each section names a problem, the cause, and what to do. Most answers start in the log: open **Settings** > **System Settings** > **Logs**, or run `docker logs organizr`. See [The log](log.md).

## Set-up and start

### I do not know the set-up code

The code is in the log of the container. Each start makes a new code, so read the newest one:

```sh
docker logs organizr 2>&1 | grep "set-up code"
```

### The container stops with "cannot write to the data folder"

The container runs as the user `65532`, not as root. The log says:

```text
Error: cannot write to the data folder /config. The user that runs Organizr must be able to write to it.
```

A folder of the host that you mount as `/config` must belong to that user:

```sh
sudo chown -R 65532:65532 /path/to/the/folder
```

A named Docker volume needs nothing. See [Installation](installation.md#the-data-folder).

### After a restore or a move, settings with passwords and keys are empty

Organizr seals the stored passwords, tokens and keys with the key in the file `secret.key`. The new container has another key. The log has a line like this one for each value:

```text
a stored setting is ignored problem=the stored secret `plexToken` cannot be opened with the key of this install
```

Put the old `secret.key` back in the data folder and start the container again. If the key is lost, enter the values again. See [Secrets](secrets.md#if-the-key-is-lost-or-wrong).

### With PostgreSQL, the log says that `seaql_migrations` already exists

At each start with PostgreSQL, the log can show lines like this one:

```text
INFO sqlx::postgres::notice: relation "seaql_migrations" already exists, skipping
```

These lines are harmless. At each start, Organizr makes sure that its tables are up to date. PostgreSQL then says that the table that records the updates is there already. You do not need to do anything.

## Reverse proxy and tabs

### The tabs open http:// addresses, but the site uses HTTPS

Organizr does not believe the headers of your reverse proxy. Add the address or the network of the proxy to `ORGANIZR_TRUSTED_PROXIES`, and start a new container. The log has this warning when the variable is missing:

```text
a request with an X-Forwarded-For header came from an address that is not a trusted proxy, so the header was ignored.
```

See [Reverse proxy](reverse-proxy.md#organizr_trusted_proxies).

### A tab shows Organizr inside Organizr

The tab has a path as its address, for example `/sonarr/`. The browser asks the host of Organizr for that path, and the proxy has no location for it, so it sends the request to Organizr. Add a location for the application to the proxy. See [Reverse proxy](reverse-proxy.md#a-tab-with-a-path).

### A tab addresses `organizr:8080` or another inside name

nginx sends the name of the upstream unless you tell it otherwise. Add `proxy_set_header Host $host;` to the location of Organizr.

### A tab stays empty, or the browser says that it refused to connect

The browser does not show the application in a frame. Two causes are common:

- The application forbids frames with `X-Frame-Options` or `frame-ancestors`.
- Organizr uses HTTPS, and the tab uses an `http://` address.

See [Reverse proxy](reverse-proxy.md#a-tab-in-a-frame). Or set the tab to open in a new browser tab.

## Login and passwords

### The e-mail with a new password does not come

1. Check that the plugin **PHP Mailer** is switched on in **Settings** > **Plugins**. Without it, the login page has no link **Forgot your password?**.
2. Open the settings of **PHP Mailer**. On the tab **Test & Options**, select **Send** next to **Send Test**. The test e-mail goes to the address of your own account, and the page shows the reason that the SMTP server gave.
3. Look for this line in the log:

   ```text
   the SMTP server did not take the e-mail: ...
   ```

   - `Connection refused` or a timeout: Organizr cannot reach the SMTP server at that address and port. Check the host name and the port. Check that the mail server runs, and that a firewall lets the container through. If the mail server is in your own network, its public address may not work from inside: use its local address.
   - A message about the login: check the user name and the password of the SMTP server.
   - A message about TLS: check the kind of encryption. Port `587` usually needs `tls`, which is STARTTLS. Port `465` needs `ssl`, which is TLS from the start.

See [E-mail](email.md).

### I forgot my password, and no e-mail comes

Another Admin can set a new password for you in **Settings** > **User Management**. A Co-Admin cannot change the password of an Admin.

Without another Admin, the e-mail is the only way: repair the e-mail settings first, then select **Forgot your password?**.

### My old password still works after a reset

That is intended. The new password from the e-mail works for 24 hours next to the old one. It becomes the password of the account when you log in with it. So a person who asks for a new password in your name cannot lock you out. See [E-mail](email.md#a-new-password).

## Settings

### Backup says that I am not authorized

Only an Admin or a Co-Admin can use the backups, because a backup holds the password hashes of every user. Log in with an account of one of these groups. See [Backups](backups.md).

### My custom CSS does nothing

Custom CSS written for PHP Organizr aims at ids and classes that these pages do not have, such as `#allHealthChecks` or `.white-box`. Change the rules to the names in [Custom CSS](custom-css.md).

### My custom JavaScript does not run

Custom JavaScript runs only when the container has `ORGANIZR_CUSTOM_JS=1`. See [Appearance](appearance.md).

## Homepage

### An item shows an error, or no data

1. Open the settings of the item and select **Test Connection**. The page shows whether the service answered, or what is wrong. See [Homepage](homepage.md#test-the-connection-of-a-service).
2. Check the address of the service. Organizr asks the service from inside its container, so `localhost` is the container of Organizr, not your server. Use the name of the container of the service, or the address of the server.
3. Run `organizr check` to ask all your services at once. See [Checking your services](checking-your-services.md).

## Ask for help

If this page does not help, open an issue in the repository. Add:

- the version of the image: `docker exec organizr organizr --version`;
- the lines of the log around the problem, without passwords, keys and tokens.
