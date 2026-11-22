# Installation

Organizr runs as one container image: `ghcr.io/tronyx/organizr-rust`. The image is for the architectures `amd64` and `arm64`. This page tells you how to start it, how to set it up, and how to update it.

## What you need

- Docker, with Docker Compose.
- A port for the web interface. The examples use port `8080`.
- A database. SQLite needs nothing more: it is a file in the data folder. For PostgreSQL, read [Choosing a database](choosing-a-database.md) first, because you choose the database before the first start.

The tag `develop` holds the newest state of the project. The project has no release yet.

## Start it with Docker Compose

1. Copy the file [compose.sqlite.yaml](../docker/examples/compose.sqlite.yaml) to your server:

   ```yaml
   services:
     organizr:
       image: ghcr.io/tronyx/organizr-rust:develop
       restart: unless-stopped
       ports:
         - "8080:8080"
       volumes:
         - organizr-data:/config

   volumes:
     organizr-data:
   ```

   For PostgreSQL, copy [compose.postgres.yaml](../docker/examples/compose.postgres.yaml) instead, and replace `change-this-password` in both places.

2. Start the container in the folder of the file:

   ```sh
   docker compose -f compose.sqlite.yaml up -d
   ```

3. Read the set-up code in the log of the container:

   ```sh
   docker compose -f compose.sqlite.yaml logs organizr
   ```

   The log has a line that ends like this one:

   ```text
   this install is not set up; the set-up wizard asks for the set-up code K7QM-2XWD-9RTB
   ```

4. Open `http://<your server>:8080` in a browser and complete the set-up wizard. See [The set-up wizard](#the-set-up-wizard).

## Start it with docker run

```sh
docker run -d --name organizr --restart unless-stopped \
  -p 8080:8080 \
  -v organizr-data:/config \
  ghcr.io/tronyx/organizr-rust:develop
```

Then read the set-up code with `docker logs organizr`.

## The set-up wizard

A new install shows the set-up wizard to everybody who opens it. The wizard has three parts:

- **Proof that this is your server**: the set-up code from the log. Only a person who can read the log of the container can set up the install. The code changes each time the container starts.
- **The Admin account**: the name, the e-mail address and the password of the first account. This account is an Admin.
- **The install**: whether you use Organizr at home or for a business, the registration password, which people need to make an account of their own, and the API key of Organizr.

After the wizard, you log in with the Admin account.

If you move from PHP Organizr, do not complete the wizard. Run the import first. See [Moving from PHP Organizr](moving-from-php.md).

## The data folder

Organizr keeps its data in the volume `/config`:

- the SQLite database, when you use SQLite;
- the key that seals the stored secrets, `secret.key`. See [Secrets](secrets.md);
- the pictures that you upload, and the backups.

Keep this volume. Without it, a new container starts as a new install.

The container runs as the user and group `65532`, not as root. A named volume, as in the examples, needs nothing more. If you use a folder of the host instead, that user must be able to write to it:

```sh
sudo mkdir -p /srv/organizr
sudo chown 65532:65532 /srv/organizr
```

Then mount the folder with `-v /srv/organizr:/config`.

## Settings of the container

Most settings are in the web interface. These are environment variables, because Organizr needs them before it can read its database:

| Variable | Meaning | Default |
| --- | --- | --- |
| `ORGANIZR_DATA_DIR` | The data folder | `/config` in the image |
| `ORGANIZR_DATABASE_URL` | The database. See [Choosing a database](choosing-a-database.md) | SQLite in the data folder |
| `ORGANIZR_LISTEN` | The address and the port that the server listens on | `0.0.0.0:8080` |
| `ORGANIZR_TRUSTED_PROXIES` | The addresses and networks of your reverse proxies, separated by commas. See [Reverse proxy](reverse-proxy.md) | None |
| `ORGANIZR_EXTENSION_MARKETPLACE` | The address of the list of a marketplace for extensions. An empty value switches the marketplace off. See [Extensions](extensions.md) | The list of the project |
| `ORGANIZR_SECRET_KEY`, `ORGANIZR_SECRET_KEY_FILE` | The key that seals the stored secrets. See [Secrets](secrets.md) | A key file in the data folder |
| `ORGANIZR_CUSTOM_JS` | `1` runs the custom JavaScript of the Admin on every page. **Warning:** the script runs in the session of every person who opens Organizr, guests too. See [Appearance](appearance.md) | Off |

## Health check

The image has a health check. Docker asks the server every 30 seconds whether it answers, and shows the container as `healthy` or `unhealthy`. You can ask the same question yourself:

```sh
docker exec organizr organizr healthcheck
```

## Update

1. Make a backup in **Settings**, **System Settings**, **Backup**. See [Backups](backups.md).
2. Get the new image and start a new container from it:

   ```sh
   docker compose -f compose.sqlite.yaml pull
   docker compose -f compose.sqlite.yaml up -d
   ```

Organizr updates its database when it starts. The data stays in the volume. [Release notes](release-notes.md) tells you what changed.

## Next steps

- [Reverse proxy](reverse-proxy.md): put Organizr behind nginx, Traefik or Caddy, with HTTPS.
- [E-mail](email.md): set up the SMTP server, so that people can ask for a new password.
- [Homepage](homepage.md): show your applications on the homepage.
