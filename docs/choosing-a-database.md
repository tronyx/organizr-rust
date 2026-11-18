# Choosing a database

Organizr stores its data in one database. It can use two database engines: SQLite and PostgreSQL. You choose the engine before you start Organizr for the first time. This page tells you how to choose, and how to make sure that Organizr uses the database you want.

## Which engine to use

Use SQLite unless you have a reason to use PostgreSQL. SQLite is the default.

| | SQLite | PostgreSQL |
|---|---|---|
| What it is | One file in the data volume | A separate database server |
| What you run | One container | Two containers: Organizr and PostgreSQL |
| What you configure | Nothing | A URL with a user name and a password |
| How you make a backup | Copy the data volume, or use the backup function of Organizr | Use the backup function of Organizr, or the tools of PostgreSQL |
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
