# Choosing a database

Organizr stores its data in one database. It can use two database engines: SQLite and PostgreSQL. You choose the engine before you start Organizr for the first time. This page tells you how to choose, and how to make sure that Organizr uses the database you want.

## Which engine to use

Use SQLite unless you have a reason to use PostgreSQL. SQLite is the default.

| | SQLite | PostgreSQL |
|---|---|---|
| What it is | One file in the data volume | A separate database server |
| What you run | One container | Two containers: Organizr and PostgreSQL |
| What you configure | Nothing | A URL with a user name and a password |
| How you make a backup | Use the [backup function](backups.md) of Organizr | Use the tools of PostgreSQL for the database, and the [backup function](backups.md) of Organizr for the files |
| Recommended for | A home server or a small team, which is almost every install | An install that already has a PostgreSQL server with backups and monitoring |

Organizr reads much more than it writes, and the data is small. SQLite is fast for this work, and it has no server that can fail or that you must update. PostgreSQL does not make Organizr faster for a normal install. It is there for people who keep all their data in one PostgreSQL server.

## How Organizr decides which database to use

Organizr reads one environment variable when it starts: `ORGANIZR_DATABASE_URL`.

- If the variable is not set, Organizr uses SQLite. It creates the file `organizr.db` in its data folder. In the container, the data folder is the volume `/config`.
- If the variable holds a URL that starts with `postgres://`, Organizr connects to that PostgreSQL server.
- If the variable holds a URL that starts with `sqlite:`, Organizr uses the SQLite file that the URL names.

Organizr creates the tables it needs when it starts. With PostgreSQL, the database that the URL names must exist, and the user of the URL must be allowed to create tables in it.

The URL for PostgreSQL holds a password. Keep it out of files that other people can read.

## Use SQLite

You do not have to set anything. Give the container a volume for `/config`, so that the database file stays when the container is replaced.

```yaml
services:
  organizr:
    image: organizr-rust:local
    volumes:
      - organizr-data:/config
    ports:
      - "8080:8080"

volumes:
  organizr-data:
```

The examples on this page use the image name `organizr-rust:local`. Replace it with the name of your image.

## Use PostgreSQL

Set `ORGANIZR_DATABASE_URL` before the first start. The URL has this form:

```text
postgres://USER:PASSWORD@HOST:PORT/DATABASE
```

This example starts PostgreSQL next to Organizr. Replace `change-this-password` in both places with a password of your own.

```yaml
services:
  organizr:
    image: organizr-rust:local
    environment:
      ORGANIZR_DATABASE_URL: postgres://organizr:change-this-password@postgres:5432/organizr
    volumes:
      - organizr-data:/config
    ports:
      - "8080:8080"
    depends_on:
      - postgres

  postgres:
    image: postgres:18-alpine
    environment:
      POSTGRES_USER: organizr
      POSTGRES_PASSWORD: change-this-password
      POSTGRES_DB: organizr
    volumes:
      - postgres-data:/var/lib/postgresql

volumes:
  organizr-data:
  postgres-data:
```

## The set-up wizard does not choose the database

The original Organizr asked for the database in its set-up wizard. This version does not. When the wizard opens, Organizr is already running on a database: the one it was started with.

There are two reasons for this. The first reason is safety. A wizard that takes database credentials is open to every visitor of a new install. It also makes the server connect to an address that the visitor names. The second reason is clarity. The environment variable is the one place where the choice is made, so you can see it and repeat it.

## Make sure that Organizr uses the database you want

Do these steps before you create the first account. After the set-up, the data is in the database that was in use.

1. Start Organizr.
2. Read the log of the server. On start-up, Organizr writes a line that says `the database is ready` and names the engine: `SQLite` or `PostgreSQL`.

   ```text
   docker logs organizr
   ```

3. Ask the server which engine it uses. The answer of `GET /api/v2/status` has the field `database`, which is `sqlite` or `postgresql`.

   ```text
   curl http://localhost:8080/api/v2/status
   ```

   The web interface is still to come. Its set-up wizard will show this value.
4. If the engine is not the one you want, stop Organizr. Then set or remove `ORGANIZR_DATABASE_URL`, and start Organizr again.

The set-up has one more safeguard. A set-up request can name a database engine in the field `driver`. If that engine is not the one the server uses, the server refuses the set-up with status `422`. The message names the engine in use. Nothing is stored.

## The set-up code

A new install has no account, so Organizr cannot know who is allowed to create the first one. To make sure that it is you, Organizr asks for a set-up code.

1. Start Organizr on an install that is not set up.
2. Read the log of the server. Organizr writes a line with the code, in this form:

   ```text
   this install is not set up; the set-up wizard asks for the set-up code K7QM-2XWD-9RTB
   ```

3. Send the code with the set-up, in the field `setupCode`. You can type small letters, and you can leave out the dashes.

Organizr makes a new code each time it starts, until the install is set up. If you restart Organizr, read the log again. After the set-up, Organizr writes no code, and the wizard is closed.

## Change the engine later

Organizr does not move data from one engine to the other when you change `ORGANIZR_DATABASE_URL`. If you change the variable on an install that is set up, Organizr starts on the other database. That database is empty, so Organizr shows the set-up wizard again. Your data is still in the first database.

To go back, set the variable to its earlier value and start Organizr again.

To keep your data when you change from SQLite to PostgreSQL, use the command in the next section.

## Move an install from SQLite to PostgreSQL

The command `organizr move-to-postgres` copies the data of an install from its SQLite file into an empty PostgreSQL database. It keeps every id. It copies all 13 tables, also the sessions, so nobody has to log in again. It copies the stored secrets as they are. They still open, because you keep the same data folder and its file `secret.key`. When the copy is done, the command counts the records in each table and compares the counts.

The command never changes the SQLite file. It reads the file `organizr.db` in the data folder. To read another file, add `--from` and the path of the file.

Before you start, check these points:

- The PostgreSQL database must hold no records. It may already have the empty tables, but it must have no rows.
- The SQLite file must come from this version. If an older version of Organizr opened the file last, the command refuses to copy it. In that case, start Organizr once on SQLite with this version, stop it, and run the command again.
- The move goes in one direction. There is no command that copies from PostgreSQL to SQLite.

The steps below use the Docker Compose file of a PostgreSQL install. Make your own file look like [compose.postgres.yaml](../docker/examples/compose.postgres.yaml). The difference with [compose.sqlite.yaml](../docker/examples/compose.sqlite.yaml) is small:

- Add the service `database`, which runs PostgreSQL.
- Add `depends_on` to the service `organizr`, so that Organizr waits for the database.
- Set `ORGANIZR_DATABASE_URL` in the service `organizr`, with the user, the password and the name of the database.
- Keep the volume `organizr-data` on `/config`. It holds the SQLite file and `secret.key`.

1. Stop Organizr, so that nothing writes to the SQLite file during the copy.
2. Change your compose file as described above. Replace `change-this-password` in both places with a password of your own.
3. Start only the database.

   ```sh
   docker compose -f compose.postgres.yaml up -d database
   ```

4. Run the command with `--dry-run`. A dry run writes nothing to PostgreSQL, not even the tables. It shows how many records the command would copy.

   ```sh
   docker compose -f compose.postgres.yaml run --rm organizr move-to-postgres --data-dir /config --dry-run
   ```

5. Read the report. Then run the same command without `--dry-run`.

   ```sh
   docker compose -f compose.postgres.yaml run --rm organizr move-to-postgres --data-dir /config
   ```

6. Start everything.

   ```sh
   docker compose -f compose.postgres.yaml up -d
   ```

7. Check that Organizr uses PostgreSQL. See [Make sure that Organizr uses the database you want](#make-sure-that-organizr-uses-the-database-you-want). Your users, tabs and settings are all there, and the set-up wizard does not open.

After the move, keep the data folder. It holds `secret.key`, and Organizr needs that file to open the stored secrets. Also keep the old file `organizr.db` until you are sure that everything works. If you want to go back to SQLite, that file is the only way: remove `ORGANIZR_DATABASE_URL`, and start Organizr again. The data that you add to PostgreSQL after the move does not come along.

A backup of an install on PostgreSQL does not hold the database. Use the tools of PostgreSQL for it. See [Backups](backups.md).
