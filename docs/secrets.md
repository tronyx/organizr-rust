# The key for stored secrets

Organizr stores secrets in its database. A secret is a value that gives access to something: the API key of Organizr, and the passwords and tokens of the services that you connect. Organizr encrypts each secret before it stores it. This page tells you where the key for that encryption is, and what you must do to keep it.

## What the key does

The key is 32 random bytes. Organizr needs it to read the secrets in the database. A person who has a copy of the database, and not the key, cannot read the secrets.

The key is not in the database. If you lose the key, the secrets in the database cannot be read again. Organizr then starts without them, and you must enter each secret again.

Organizr does not encrypt the rest of the database. The names of users, the addresses of tabs and the other settings are stored as plain text. The password of a user is stored as a hash, which is a value that proves a password and does not show it.

## Where the key is

Organizr reads the key when it starts. It looks in two places, in this order:

1. The environment variable `ORGANIZR_SECRET_KEY`. Its value is the key as 64 hex digits.
2. The key file. By default this is the file `secret.key` in the data folder. In the container, the data folder is the volume `/config`.

If the variable is not set and the key file does not exist, Organizr makes a new key. It writes the key to the key file and says so in its log. Only the user that runs Organizr can read the file.

To use another file, set the environment variable `ORGANIZR_SECRET_KEY_FILE` to its path.

## Keep a copy of the key

Make a copy of the key file when you set Organizr up. Keep the copy with your backups of the database.

- If you use SQLite, the key file is in the same volume as the database. A backup of the whole volume holds both.
- If you use PostgreSQL, the database is on another server. A backup of that server does not hold the key. Make a backup of the data volume of Organizr too.

A backup that holds the database and the key in one place is as open as a database without encryption. If that matters for you, keep the key in another place than the database backup.

## If the key is lost or wrong

Organizr starts also when it cannot read a stored secret. It writes one line to its log for each such secret:

```text
a stored setting is ignored problem=the stored secret `plexToken` cannot be opened with the key of this install
```

The setting in the line is empty until you enter its value again. The API key of Organizr is such a secret too. Scripts that use the old API key get the answer `401` until you set a new one.

Do this to repair the install:

1. Make sure that the key file is the one that belongs to the database. If you have a backup of the key file, put it back and start Organizr again.
2. If the key is lost, log in with your user name and your password. Passwords of users do not need the key.
3. Enter each secret again in the settings.

## How to give the key in the environment

Use the environment variable if you do not want the key in the data volume. Make the key yourself, for example with this command:

```sh
openssl rand -hex 32
```

Put the result in `ORGANIZR_SECRET_KEY`. Organizr then does not read or write a key file. Use the same value at each start.

Do not change the key of an install that has stored secrets. Organizr cannot read the secrets that it stored with another key.
