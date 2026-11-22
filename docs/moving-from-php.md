# Moving from PHP Organizr

This page is a checklist for a move from PHP Organizr 2.1 to this version. Each step links to the page with the details.

The move never changes the old install. You can run both side by side, and switch the reverse proxy to the new install when it is ready.

## 1. Before the move

- Make a copy of the folder of the old install, or stop the old Organizr during the import.
- Check the database of the old install. The import reads SQLite only. An old install with MySQL or PostgreSQL cannot be imported.
- Write down what you changed by hand: custom CSS, custom JavaScript, custom HTML items, and files that you added in `data/pages`, `data/routes` or `data/plugins`. This version cannot run PHP files, so those do not come along.
- Choose the database of the new install: SQLite or PostgreSQL. See [Choosing a database](choosing-a-database.md).

## 2. Import

Run the import before the first start of the new install. Do not complete the set-up wizard: the import makes the install.

```sh
docker run --rm \
  -v organizr-data:/config \
  -v /path/to/old/config:/old:ro \
  ghcr.io/tronyx/organizr-rust:develop \
  import --from /old --data-dir /config --dry-run
```

Read the report of the dry run, then run the same command without `--dry-run`. [Importing](importing.md) tells you what the import takes, what it repairs, and what the report means.

The import brings the users with their passwords and second factors, the groups, categories and tabs, the settings, the API key, the pictures, and the records of the bookmark, invite and chat plugins. Every user logs in again, with the old password.

After the import, make a copy of the file `secret.key` in the data folder, at a safe place. See [Secrets](secrets.md).

## 3. Start the new install

Start the container with the same volume and the same database settings as the import. See [Installation](installation.md).

Organizr listens on port `8080` in the container, and runs as the user `65532`, not as root.

## 4. The reverse proxy

- Point the proxy of Organizr at the new container, port `8080`.
- Add the address of the proxy to `ORGANIZR_TRUSTED_PROXIES`. Without it, the tabs and links use `http://`, and the log shows the proxy as the caller.
- Tabs with a path, such as `/sonarr/`, need a location for each application in the proxy of Organizr. Without it, the tab shows Organizr inside Organizr.

[Reverse proxy](reverse-proxy.md) has examples for nginx, Traefik and Caddy.

The forward auth routes of PHP Organizr work as before, also the old forms such as `/api/v2/auth?group=1`. One difference: Organizr believes the forwarding headers of a trusted proxy only. See [Forward auth](forward-auth.md).

## 5. Appearance

- **Themes**: the themes of the marketplace are rebuilt for this version. Install your theme again in **Settings**, **Customize**, **Marketplace**.
- **Custom CSS**: the pages use other ids and classes. Change your custom CSS once, with the table in [Custom CSS](custom-css.md).
- **Custom JavaScript**: it runs only when the container has `ORGANIZR_CUSTOM_JS=1`. See [Appearance](appearance.md).

## 6. Homepage

- **Custom HTML items** run in a closed frame. A script in an item cannot reach the page of Organizr or its cookies. If an item needs that, switch on **Trust custom HTML 01** (with the number of the item) in its settings. See [Homepage](homepage.md#trusted-items).
- Links in custom HTML to the pop-ups of PHP, such as `#new-request`, open the matching search or page.
- Test the connection of each item with its button **Test Connection**, in **Settings**, **Homepage Items**. See [Homepage](homepage.md#test-the-connection-of-a-service).

## 7. Things that work differently

- **Scheduled jobs** run inside the server. Remove the cron entry for `cron.php`. See [Scheduled jobs](scheduled-jobs.md).
- **Backups** are for an Admin only, because a backup holds the password hashes of every user. A Co-Admin sees a note instead. See [Backups](backups.md).
- **Password recovery** sends a new password that works for 24 hours, next to the old password. The old password stays good until somebody logs in with the new one. See [E-mail](email.md).
- **Plugins of other authors** are WebAssembly extensions, not PHP files. See [Extensions](extensions.md).
- **The log** goes to the output of the container and to the folder `logs` in the data folder. See [The log](log.md).

[Release notes](release-notes.md) lists the differences that you are most likely to notice, and [Deviations](deviations.md) lists every one, with its reason.

## 8. When everything works

- Make a backup of the new install. See [Backups](backups.md).
- Keep the old install and its database for a while. The import leaves the table `options` and the PHP files that you added in the old install.
