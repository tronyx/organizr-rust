# 🧳 Moving from PHP Organizr

[← All documentation](README.md)

This is your checklist for moving from PHP Organizr 2.1 to this version, from the import to the reverse proxy. Each step links to the page with the details.

The move never changes your old install. You can run both side by side, and switch your reverse proxy to the new one when it's ready.

> [!IMPORTANT]
> Run the import **before** you open the new install in a browser. Don't complete the set-up wizard: the import builds the install for you, and it refuses a database that already holds records.

> [!CAUTION]
> **Breaking changes.** Some things that worked in PHP Organizr work differently here, for safety. Check each one that your install uses. The import ends with the list of the ones it finds in your data, under **Breaking changes for this install**, and an Admin sees the same list at the top of the page after the login, until an Admin hides it (D-158).
>
> - **Service addresses that are only a path**, such as `/sonarr`, work only on the hosts in **Allowed Hosts** (or `ORGANIZR_ALLOWED_HOSTS`). The first Admin login stores the host it used while the list is empty. Add any other host that you reach Organizr at. See [step 5](#-5-homepage) (D-154, D-155).
> - **Forward auth**: when the path names a group, such as `/api/v2/auth-1`, a `group=` in the query never changes it. An allow list in the query counts only when your proxy writes it into the address itself, as nginx does. See [Forward auth](forward-auth.md) (D-136, D-157).
> - **Admin and Co-Admin accounts linked to OpenID Connect** keep their link only when the provider sends the account's own e-mail address, marked as checked. Otherwise, log in with the password once and link the account again (D-148, D-156).
> - **Co-Admins** no longer change trusted custom HTML items, the sign-in settings, OpenID Connect, the services of the health checks, single sign-on, or the allowed hosts and trusted proxies. Only an Admin does (D-131 to D-135, D-155).
> - **E-mail links** name the site from the setting **Domain** of the e-mail settings, never from the request. The first Admin login fills it in when it's empty (D-138).
> - **Komga single sign-on with a master password** asks Komga with the e-mail address of the Plex account after a Plex login (D-150).
> - **The donation history** of the Donate item shows only the payments that this version marked, not the ones from before the move (D-152).
> - **Invites for Emby** make an Emby user only while the invites plugin is set to Emby (D-149).
> - **A new password ends the account's other sessions** (D-146).
> - **Scripts** that call `GET /plugins/bookmark/setup/…`, `GET /plugins/plexlibraries/setup/tab` or the test e-mail route from another site are refused (D-144).
>
> [Deviations](deviations.md) has the reason for each.

## 📋 1. Before the move

- 🗄 **Check the old database.** The import reads SQLite only. An old install on MySQL or PostgreSQL can't be imported.
- 📝 **Write down what you changed by hand**: custom CSS, custom JavaScript, custom HTML items, and files you added in `data/pages`, `data/routes` or `data/plugins`. This version can't run PHP files, so those don't come along.
- 🧭 **Choose the database of the new install**: SQLite or PostgreSQL. See [Choosing a database](choosing-a-database.md). Not sure? SQLite.
- 📂 **Find the old folder.** If PHP Organizr ran in Docker (the `organizr/organizr` or linuxserver image), it's the host folder you mounted as `/config`: the one that holds `www/`. On a server without Docker, it's the folder with `index.php` and `data/`. Below, it's `/path/to/old/config`.

## 🚚 2. Import

These steps use Docker Compose with SQLite. For PostgreSQL, see [With PostgreSQL](#-with-postgresql) below.

1. Stop the old Organizr, or make a copy of its folder. A database that changes during the import can give an incomplete result.
2. Make a folder for the new install, and save [compose.sqlite.yaml](../docker/examples/compose.sqlite.yaml) in it as `compose.yaml`. Don't start it yet.

   ```bash
   mkdir organizr && cd organizr
   ```

3. Do a dry run. It reads the old install, prints a report, and writes nothing:

   ```bash
   docker compose run --rm -v /path/to/old/config:/old:ro organizr import --from /old --dry-run
   ```

   `/path/to/old/config` is the old folder from step 1 of the checklist. `:ro` mounts it read-only, so nothing can change it.

   > [!TIP]
   > **`permission denied`?** The import runs as the user `65532`, and PHP Organizr often lets only its owner into `www/db`. The error starts with `cannot look at /old/www/db/...: Permission denied (os error 13)`. ([Importing](importing.md#-if-you-see-permission-denied) has the whole message.) Let that one user in, just for the import:
   >
   > ```bash
   > sudo setfacl -R -m u:65532:rX /path/to/old/config/www/db
   > ```
   >
   > Take it away again after the import with `sudo setfacl -R -x u:65532 /path/to/old/config/www/db`. No `setfacl`? Install the `acl` package, or use `sudo chmod o+rx /path/to/old/config/www/db` and undo it afterward with `sudo chmod o-rx /path/to/old/config/www/db`: that folder holds your password hashes. If the database file itself isn't world-readable, `setfacl` is the way.

4. Read the report. A healthy one looks like this:

   ```text
   Dry run: nothing was written.

   These records would be imported:
          6  groups
         14  users
          5  categories
         23  tabs
          2  bookmark categories
         11  bookmarks
          3  invites
        120  chat messages
         58  settings that differ from the default
          9  uploaded pictures
          2  other files

   Sessions are not imported (31 found). Every user logs in again, with the same password.

   !!! Breaking changes for this install !!!
   These worked in PHP Organizr and work differently now:
     - Service addresses that are only a path work only on the hosts in Allowed Hosts (...). These settings hold such an address: `sonarrURL` (/sonarr).
   Check your reverse proxy too: the forward auth checks changed (D-136, D-157). "Moving from PHP Organizr" in the docs lists every breaking change.

   Repairs:
     - The tab "plex" is now called "plex (2)", because another tab has the same name.

   Tables that this version does not import yet (their rows stay in the old database):
     - options: 3 rows
   ```

   Check that the counts match your install, and read every line under **Breaking changes for this install**, **Repairs**, **Left out** and **To look at**. A few repairs are normal: PHP checked less than this version. [The report](importing.md#-the-report) explains each part.

5. Run the import for real: the same command without `--dry-run`.

   ```bash
   docker compose run --rm -v /path/to/old/config:/old:ro organizr import --from /old
   ```

   It ends with `The import is complete.` and the same report. Keep it: it lists what to look at afterward.

6. Start the new install:

   ```bash
   docker compose up -d
   ```

   Organizr listens on port `8080` in the container, and runs as the user `65532`, not as root.

7. Copy the new key for the stored secrets to a safe place, away from your backups:

   ```bash
   docker compose cp organizr:/config/secret.key ./secret.key
chmod 600 ./secret.key
   ```

   Without this file, the passwords and tokens in the database can't be read again. See [Secrets](secrets.md).

8. Open `http://<your server>:8080` and log in with your old user name and password. The set-up wizard doesn't appear: the import already set the install up.

The import brings over the users with their passwords and second factors, the groups, categories and tabs, the settings, the API key, the pictures, and the records of the bookmark, invite and chat plugins. Every user logs in again, with their old password. [Importing](importing.md) lists exactly what comes along and what it repairs.

### 🐘 With PostgreSQL

Save [compose.postgres.yaml](../docker/examples/compose.postgres.yaml) as `compose.yaml` and set the password in both places. Then run the same steps 3 to 8. `docker compose run` starts the database first and waits until it's ready, so the import writes straight into PostgreSQL.

Keep the volume `<project>_organizr-data` with PostgreSQL too: it holds `secret.key` and the pictures.

### 🐳 Without Compose

If you start Organizr with `docker run -v organizr-data:/config`, run the import with the same volume:

```bash
docker run --rm \
  -v organizr-data:/config \
  -v /path/to/old/config:/old:ro \
  ghcr.io/tronyx/organizr-rust:develop \
  import --from /old --dry-run
```

> [!WARNING]
> Use the volume that your server will mount. Compose calls it `<project>_organizr-data` (for example `organizr_organizr-data`), not `organizr-data`. An import into the wrong volume looks fine, but the server then starts empty and shows the set-up wizard. `docker volume ls` lists the names.

### 🆘 You already completed the wizard

Then the database holds records, and the import stops with `the new database is not empty; an import needs a new, empty database, so nothing was changed`. If the new install holds nothing you want to keep, delete it and start over:

```bash
docker compose down -v
```

This deletes the new install's volumes (with PostgreSQL, the database too). Your old install isn't touched. Then go back to step 3.

## 🌐 3. The reverse proxy

- Point your proxy at the new container, port `8080`.
- Add the proxy's address to `ORGANIZR_TRUSTED_PROXIES`. Without it, tabs and links use `http://`, and the log shows the proxy as the caller. [Trusted proxies](reverse-proxy.md#-organizr_trusted_proxies) shows how to find the address.
- Tabs with a path, such as `/sonarr/`, need a location for each application in your proxy. Without it, the tab shows Organizr inside Organizr.
- Once the proxy works, remove the `ports:` lines from `compose.yaml`, so nobody reaches Organizr around the proxy.

[Reverse proxy](reverse-proxy.md) has examples for nginx, Nginx Proxy Manager, SWAG, Traefik and Caddy.

The forward auth routes of PHP Organizr work as before, also the old forms such as `/api/v2/auth?group=1`. One difference: Organizr believes the forwarding headers of a trusted proxy only. See [Forward auth](forward-auth.md).

## 🎨 4. Appearance

- **Layout**: an imported install stays on the OG Organizr layout, so nothing changes for your users until you pick another. See [Appearance](appearance.md).
- **Themes**: the marketplace themes are rebuilt for this version. Install yours again in **Settings › Customize › Marketplace**.
- **Custom CSS**: the pages use other ids and classes. Change your custom CSS once, with the table in [Custom CSS](custom-css.md).
- **Custom JavaScript**: it runs only when the container has `ORGANIZR_CUSTOM_JS=1`. See [Appearance](appearance.md).

## 🏠 5. Homepage

- **Custom HTML items** run in a closed frame. A script in an item can't reach Organizr's page or its cookies. If an item needs that, switch on **Trust custom HTML 01** (with the number of the item) in its settings. Trust only HTML you wrote: a trusted item runs with the session of everyone who opens the homepage. See [Homepage](homepage.md).
- Links in custom HTML to the pop-ups of PHP, such as `#new-request`, open the matching search or page.
- Test each item with its **Test Connection** button, in **Settings › Tab Editor › Homepage Items**. See [Homepage](homepage.md).
- **Service addresses that are only a path**, such as `/sonarr`, work only on the hosts you list in [`ORGANIZR_ALLOWED_HOSTS`](configuration.md), for example `organizr.example.com,192.168.1.10:8080`. Add the names you reach Organizr at, or enter full addresses, such as `http://sonarr:8989`. The **Test Connection** button says so for such an address. This is for the addresses of items, login services and single sign-on, not for tabs (D-137, D-154).

## 🔀 6. Things that work differently

- ⏰ **Scheduled jobs** run inside the server. Remove the cron entry for `cron.php`. See [Scheduled jobs](scheduled-jobs.md).
- ✉ **Password recovery** sends a new password that works for 24 hours, next to the old one. The old password keeps working until somebody logs in with the new one. See [E-mail](email.md).
- 🧩 **Plugins of other authors** are WebAssembly extensions, not PHP files. See [Extensions](extensions.md).
- 📜 **The log** goes to the container output and to the folder `logs` in the data folder. See [The log](log.md).
- 🔐 **Admin and Co-Admin accounts linked to OpenID Connect** keep their link only when the provider sends the account's own e-mail address, marked as checked. Otherwise, log in with the password once, then link the account in your settings. Other accounts keep their link. See [OpenID Connect](openid-connect.md) (D-148, D-156).

[Release notes](release-notes.md) lists the differences you're most likely to notice, and [Deviations](deviations.md) lists every one, with its reason.

## ✅ 7. When everything works

- 💾 Make a backup of the new install. See [Backups](backups.md). With PostgreSQL, also save the database with `pg_dump`; see [Choosing a database](choosing-a-database.md#-back-up-postgresql).
- 🔌 Take the old install off the network: stop it, or remove it from your proxy. It still holds the same password hashes and API key, and the secrets of your services under PHP's weaker encryption.
- 🧊 Keep the old folder for a while as a cold copy, readable only by you. The import leaves the table `options` and the PHP files you added there.
- 🔑 If the old API key was ever in a URL or a shared script, make a new one in **Settings › System Settings › General** (**Generate New API Key**).
- 🔒 Undo the `setfacl` or `chmod` from step 3, if you used it.
