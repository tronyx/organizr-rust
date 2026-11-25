# 🗄️ Choosing a database

[← All documentation](README.md)

Organizr keeps its data in one database: SQLite or PostgreSQL. You choose before the first start. This page helps you choose, sets each one up, and moves an install from SQLite to PostgreSQL.

> [!TIP]
> Not sure? Use SQLite, the default, and skip this page. You can still move to PostgreSQL later.

## 🤔 Which one to use

| | 🪶 SQLite | 🐘 PostgreSQL |
|---|---|---|
| What it is | One file in the data volume | A separate database server |
| What you run | One container | Two containers: Organizr and PostgreSQL |
| What you configure | Nothing | A URL with a user name and a password |
| How you back it up | Organizr's [backup](backups.md) holds it | `pg_dump` for the database, plus Organizr's [backup](backups.md) for the files |
| Good for | A home server or a small team: almost every install | An install where you already run PostgreSQL with backups and monitoring |

Organizr reads far more than it writes, and its data is small. SQLite is fast for that, and it has no server that can fail or that you must update. PostgreSQL doesn't make a normal install faster. It's there for people who keep all their data in one PostgreSQL server.

## 🔍 How Organizr picks the database

Organizr reads one environment variable when it starts: `ORGANIZR_DATABASE_URL`.

| Value | Database |
| --- | --- |
| Not set | SQLite, in the file `organizr.db` in the data folder. In the container, that's the volume `/config` |
| `postgres://...` or `postgresql://...` | That PostgreSQL server |
| `sqlite:<file>` | The SQLite file that the URL names |

Organizr creates and updates its tables when it starts. With PostgreSQL, the database in the URL must exist, and its user must be allowed to create tables in it. The official `postgres` image does both for you with `POSTGRES_USER` and `POSTGRES_DB`.

The URL holds a password, and anyone who can run `docker inspect` can read it. Keep it out of files you share or commit: put it in a `.env` file next to `compose.yaml`, readable only by you (`chmod 600 .env`).

## 🪶 Use SQLite

There's nothing to set. Give the container a volume for `/config`, so the database survives when the container is replaced. [compose.sqlite.yaml](../docker/examples/compose.sqlite.yaml) does exactly that:

```yaml
services:
  organizr:
    image: ghcr.io/tronyx/organizr-rust:develop
    restart: unless-stopped
    volumes:
      - organizr-data:/config
    ports:
      - "8080:8080"

volumes:
  organizr-data:
```

Keep the data folder on a local disk. SQLite on a network share (NFS, SMB) often ends in `database is locked` errors or a damaged database, and Organizr doesn't detect or refuse such a share: use a local disk for `/config` with SQLite.

## 🐘 Use PostgreSQL

Set `ORGANIZR_DATABASE_URL` before the first start. The URL has this form:

```text
postgres://<user>:<password>@<host>:<port>/<database>
```

A password with special characters such as `@`, `:` or `/` must be percent-encoded in the URL (`@` becomes `%40`). A password from `openssl rand -hex 24` needs no encoding.

This is [compose.postgres.yaml](../docker/examples/compose.postgres.yaml): Organizr with PostgreSQL next to it. Replace `change-me-database-password` in **both** places with your own password, made with `openssl rand -hex 24`.

```yaml
services:
  organizr:
    image: ghcr.io/tronyx/organizr-rust:develop
    restart: unless-stopped
    depends_on:
      database:
        condition: service_healthy
    ports:
      - "8080:8080"
    environment:
      ORGANIZR_DATABASE_URL: postgres://organizr:change-me-database-password@database:5432/organizr
    volumes:
      - organizr-data:/config

  database:
    image: postgres:18-alpine
    restart: unless-stopped
    environment:
      POSTGRES_USER: organizr
      POSTGRES_PASSWORD: change-me-database-password
      POSTGRES_DB: organizr
    volumes:
      - organizr-database:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready --username organizr --dbname organizr"]
      interval: 5s
      timeout: 5s
      retries: 10

volumes:
  organizr-data:
  organizr-database:
```

Organizr connects once when it starts. The health check with `condition: service_healthy` makes it wait until PostgreSQL is ready, and `restart: unless-stopped` brings it back if the database was slow anyway.

Keep the volume `organizr-data` on `/config` with PostgreSQL too: it holds `secret.key` and the uploaded pictures.

The project tests with PostgreSQL 18, as in the example.

> [!WARNING]
> **PostgreSQL on another computer?** Add `?sslmode=require` to the URL, for example `postgres://organizr:<password>@db.example.com:5432/organizr?sslmode=require`. Without it, the password and your data (password hashes and encrypted secrets included) can cross the network unencrypted. `sslmode=require` encrypts the connection but doesn't check the server's certificate; `sslmode=verify-full` also checks it, when the certificate is signed by an authority the container trusts.

## 🧙 The wizard doesn't choose the database

PHP Organizr asked for the database in its set-up wizard. This version doesn't: when the wizard opens, Organizr already runs on the database it was started with.

There are two reasons:

- 🔒 **Safety.** A wizard that takes database credentials is open to every visitor of a new install, and it makes the server connect to any address a visitor names.
- 🔎 **Clarity.** The environment variable is the one place where the choice is made, so you can see it and repeat it.

## ✅ Make sure Organizr uses the database you want

Check this before you complete the set-up wizard. After the set-up, the data lives in whichever database was in use.

1. Start Organizr.
2. Open the set-up wizard. Under the form, it says: "The server stores its data in **SQLite**" (or **PostgreSQL**).
3. Or read the log. At start, Organizr writes `the database is ready` with the engine, `SQLite` or `PostgreSQL`:

   ```bash
   docker compose logs organizr | grep "database is ready"
   ```

4. Or ask the server. The answer of `GET /api/v2/status` has the field `database`, which is `sqlite` or `postgresql`:

   ```bash
   curl http://localhost:8080/api/v2/status
   ```

5. Wrong engine? Stop Organizr, set or remove `ORGANIZR_DATABASE_URL`, and start it again.

One more safeguard: a set-up request can name an engine in the field `driver`. If that's not the engine the server uses, the server refuses the set-up with status `422`, names the engine in use, and stores nothing.

## 🔢 The set-up code

A new install has no account yet, so Organizr can't know who may make the first one. To be sure it's you, it asks for a set-up code that only the log shows:

```text
this install is not set up; the set-up wizard asks for the set-up code K7QM-2XWD-9RTB
```

Enter it in the field **Set-up code** of the wizard (scripts send it in the field `setupCode`). Small letters and missing dashes are fine. Organizr makes a new code at each start until the install is set up; after a restart, read the log again. Once set up, there's no code, and the wizard is closed. [Installation](installation.md#-the-set-up-wizard) walks through the whole wizard.

## 🔀 Change the engine later

Changing `ORGANIZR_DATABASE_URL` doesn't move any data. On an install that's set up, Organizr simply starts on the other database. That one is empty, so you see the set-up wizard again, while your data waits in the first database.

To go back, set the variable to its old value and start Organizr again. To take your data along from SQLite to PostgreSQL, use the command in the next section.

## 🚚 Move from SQLite to PostgreSQL

`organizr move-to-postgres` copies your install from its SQLite file into an empty PostgreSQL database:

- 🆔 It keeps every id, and copies every table, sessions included, so nobody has to log in again.
- 🔑 It copies the encrypted secrets as they are. They still open, because you keep the same data folder and its `secret.key`.
- 🔢 When the copy is done, it counts the records in each table on both sides and compares the counts.
- 🔒 It never changes the SQLite file. By default it reads `organizr.db` in the data folder; `--from <file>` names another file.

Before you start:

- The PostgreSQL database must hold no records. Empty tables are fine; rows are not.
- The SQLite file must come from this version. If an older version opened it last, the command refuses. Start Organizr once on SQLite with this version, stop it, and run the command again.
- The move goes one way. There's no command that copies from PostgreSQL back to SQLite.

Run these in the folder of your `compose.yaml`:

1. Stop Organizr, so nothing writes to the SQLite file during the copy:

   ```bash
   docker compose stop organizr
   ```

2. Change `compose.yaml` to look like [compose.postgres.yaml](../docker/examples/compose.postgres.yaml). The difference from the SQLite file is small:

   - Add the service `database`, which runs PostgreSQL, and its volume `organizr-database`.
   - Add `depends_on` to the service `organizr`, so Organizr waits for the database.
   - Set `ORGANIZR_DATABASE_URL` in the service `organizr`, with the user, the password and the name of the database.
   - Keep the volume `organizr-data` on `/config`. It holds the SQLite file and `secret.key`.

   Replace `change-me-database-password` in both places with your own password.

3. Start only the database:

   ```bash
   docker compose up -d database
   ```

4. Do a dry run. It writes nothing to PostgreSQL, not even the tables, and shows how many records it would copy:

   ```bash
   docker compose run --rm organizr move-to-postgres --dry-run
   ```

   The report starts with `Dry run: nothing was written.` and lists each table with its number of records.

5. Check the counts, then copy for real:

   ```bash
   docker compose run --rm organizr move-to-postgres
   ```

6. Start everything:

   ```bash
   docker compose up -d
   ```

7. Check that Organizr now uses PostgreSQL, as in [Make sure Organizr uses the database you want](#-make-sure-organizr-uses-the-database-you-want). Your users, tabs and settings are all there, and the set-up wizard doesn't open.

After the move, keep the data folder: it holds `secret.key`, which Organizr needs to open the stored secrets. Also keep the old `organizr.db` until you're sure everything works. It's your only way back to SQLite: remove `ORGANIZR_DATABASE_URL` and start Organizr again. Anything you added in PostgreSQL after the move stays behind.

## 💾 Back up PostgreSQL

Organizr's backup of a PostgreSQL install holds the files of the data folder, but not the database. Save the database with PostgreSQL's own tools, right after an Organizr backup, so the two match.

Save it (run in the folder of your `compose.yaml`):

```bash
docker compose exec -T database pg_dump --username organizr --format custom organizr > organizr-database.dump
```

Put it back:

1. Stop Organizr: `docker compose stop organizr`.
2. If you also restore an Organizr backup, do that first: see [Backups](backups.md).
3. Empty the database, so nothing of a newer version stays behind (this matters for a rollback):

   ```bash
   docker compose exec database dropdb --username organizr --maintenance-db postgres organizr
   docker compose exec database createdb --username organizr --maintenance-db postgres organizr
   ```

4. Restore the dump into the empty database:

   ```bash
   docker compose exec -T database pg_restore --username organizr --dbname organizr < organizr-database.dump
   ```

5. Start Organizr: `docker compose up -d`.

The dump holds password hashes and encrypted secrets. Store it as carefully as your backups, and away from `secret.key`.
