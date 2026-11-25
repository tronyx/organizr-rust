# 🚚 Importing an install of PHP Organizr

[← All documentation](README.md)

The `organizr import` command moves an install of PHP Organizr into this version. This page is the reference: what the import needs, what it takes, what it repairs, and what its report means. For the step-by-step move, start with [Moving from PHP Organizr](moving-from-php.md).

## 💡 The short version

- 🔒 The import only **reads** the old install. It never changes it, and it never runs PHP code.
- 🆕 It writes a **new** install, once, before the first start of the new server. The new database must be empty, so don't complete the set-up wizard first.
- 👀 `--dry-run` shows the report and writes nothing. Always start with it.
- 👥 Users keep their passwords and second factors. Everybody logs in again, because sessions don't come along.
- 🔑 Afterward, back up the new `secret.key`: it's the only key to the passwords and tokens that the import stored.

## 📋 What the import needs

- **The folder of the old install.** That's the folder of the PHP application, which holds `index.php` and the folder `data`. For a container image with a `/config` volume, you can give the volume folder. The import then looks in `www/organizr`, `www/Dashboard`, `www` and `html`.
- **The SQLite database file of the old install.** The old configuration file names the file by its path on the old server. The import looks for the file at that path, and at the end of that path below the folder you gave. If it doesn't find the file, name it with `--database`.
- **A data folder for the new install** that's empty or doesn't exist yet.

This version imports an old install that uses SQLite. It refuses one that uses MySQL or PostgreSQL (`this version imports SQLite databases only`). The new install can use SQLite or PostgreSQL; [Choosing a database](choosing-a-database.md) tells you how to pick.

## 🚀 Import the old install

In these commands, `/path/to/old/config` is the old folder. Run them in the folder that holds your new `compose.yaml`, before you ever run `docker compose up`.

1. Stop the old Organizr, or copy its folder and its database file. A database that changes during the import can give an incomplete result.
2. Do a dry run. It reads the old install, prints the report, and writes nothing:

   ```bash
   docker compose run --rm -v /path/to/old/config:/old:ro organizr import --from /old --dry-run
   ```

3. Read the report. [The report](#-the-report) explains each part, and [Moving from PHP Organizr](moving-from-php.md#-2-import) shows what a healthy one looks like.
4. Run the import: the same command without `--dry-run`.

   ```bash
   docker compose run --rm -v /path/to/old/config:/old:ro organizr import --from /old
   ```

5. Keep the report. It lists what you must look at after the import.
6. Start the server with `docker compose up -d`, and log in.
7. Back up `secret.key` from the data folder: `docker compose cp organizr:/config/secret.key ./secret.key`, then `chmod 600 ./secret.key`. [Secrets](secrets.md) tells you why the stored secrets can't be read without it.

`docker compose run` uses the same image, volume and settings as your server, so the import writes to the right place: the volume `<project>_organizr-data` and, with PostgreSQL, the database of `ORGANIZR_DATABASE_URL`.

### 🔐 If you see permission denied

The container runs the import as the user `65532`. That user must be able to enter the folders of the old install and read its files. In PHP Organizr, often only the owner can enter `www/db`. The import then stops and names the path:

```text
cannot look at /old/www/db/<database file>: Permission denied (os error 13); the user that runs the import must be able to enter each folder of that path (in the container image, that user has the UID 65532)
```

Let only that user in, for the import:

```bash
sudo setfacl -R -m u:65532:rX /path/to/old/config/www/db
```

After the import, take it back:

```bash
sudo setfacl -R -x u:65532 /path/to/old/config/www/db
```

No `setfacl` on your system? Install the `acl` package, or use `sudo chmod o+rx /path/to/old/config/www/db` and undo it with `sudo chmod o-rx /path/to/old/config/www/db`. Don't leave it open: that folder holds the password hashes and your services' secrets. If the database file itself isn't world-readable, `chmod` on the folder isn't enough, and `setfacl` is the way.

### 🐳 With docker run, or without Docker

If you start the server with `docker run -v organizr-data:/config`, give the import the same volume:

```bash
docker run --rm \
  -v organizr-data:/config \
  -v /path/to/old/config:/old:ro \
  ghcr.io/tronyx/organizr-rust:develop \
  import --from /old
```

> [!WARNING]
> A Compose install doesn't use the volume `organizr-data`; it uses `<project>_organizr-data`. Check with `docker volume ls`. An import into the wrong volume looks fine, but the server then starts empty.

Running the `organizr` program without Docker? Use the same data folder, database and secret key options that you'll use for `organizr serve` (see [Configuration](configuration.md#-commands)):

```bash
organizr import --from /path/to/old/organizr --dry-run
organizr import --from /path/to/old/organizr --data-dir /config
```

### 🔁 If the import fails or must run again

- If the import fails, it stores no records. Fix the cause and run it again. Files that a failed import copied stay in the data folder, and the next import writes over them.
- If the new database holds records, the import stops and changes nothing (`the new database is not empty`). To import again, start with a new data folder or a new, empty database. With Compose, `docker compose down -v` deletes the new install's volumes.

## 📦 What is imported

| From the old install | In the new install |
|---|---|
| Groups, users, categories, tabs | The same records, with the same ids |
| Bookmarks and bookmark categories of the bookmark plugin | The same records, with the same ids |
| Invites of the invites plugin | The same records, with the same ids. A code that wasn't used still works. An invite without a code is left out. So is an invite whose code another invite has in another letter case. The report lists each one |
| Messages of the chat plugin (table `chatroom`) | The same records, with the same ids. PHP stored the markup of a message as HTML entities; they're read back once, so the message is the text that the person wrote. A message without text is left out, and the report lists it |
| User passwords | The same hashes. Each user logs in with the old password |
| A user's second factor and Plex token | Kept, encrypted with the key of the new install |
| Settings in `data/config/config.php` | The settings of the new install. A setting that has its default value isn't stored |
| Values that PHP encrypted (keys that hold the word "password") | Decrypted with the `organizrHash` of the old install, and encrypted with the key of the new install |
| API key | Kept, encrypted. Scripts that use the key keep working |
| Registration password | Kept as a hash |
| Uploaded pictures in `data/userTabs` | Copied to `userTabs` in the data folder |
| Pictures someone added to `plugins/images` | Copied to `userTabs` in the data folder. See [Pictures](#-pictures) |
| Custom favicon files in `data/favicon` | Copied to `favicon` in the data folder |

## 🚫 What is not imported

- **Sessions.** Every user logs in again.
- **The table `options`.** The report gives its number of rows. The rows stay in the old database, so keep that file.
- **The picture cache** in `data/cache`. Organizr builds its cache again.
- **PHP files** that someone added in `data/pages`, `data/routes` and `data/plugins`. The report lists them. This version can't run PHP.
- **These settings**: `driver`, `dbHost`, `dbUsername`, `dbPassword`, `dbLocation`, `dbName`, `logLocation`, `configVersion` and `organizrHash`. The new install gets its database from `ORGANIZR_DATABASE_URL`, writes its log to the standard output and to the folder `logs` of its data folder (see [The log](log.md)), and encrypts secrets with its own key.

## 📇 Settings this version doesn't know

An old configuration file can hold settings that this version doesn't know, for example the settings of a plugin that isn't part of Organizr, or of another Organizr version. The import keeps each one in the table `legacy_settings`, encrypted with the key of the new install. The report lists their names. Nothing reads these settings at this time.

## 🌄 Pictures

PHP Organizr has picture folders below `plugins/images`, and people put their own pictures there. This version ships only Organizr's own pictures. So the import copies these to the uploaded pictures:

- Each picture below `plugins/images` that a group, a user, a category, a tab or a setting uses, if this version doesn't ship it.
- Each other picture in `plugins/images/tabs` that this version doesn't ship. The picture chooser then keeps showing them.

A record that used such a picture then points to the new place, `data/userTabs/<name>`. If an uploaded picture has the same name, the copied one gets its old folder's name in front, for example `tabs-logo.png`.

The import copies GIF, PNG, JPEG and SVG files. The report names each picture that a record uses and that the import can't copy.

## 🔧 Repairs

PHP Organizr checks less than this version, so its database can hold records that this version can't hold. The import repairs each one and writes one line about it in the report.

| In the old install | In the new install |
|---|---|
| Two groups, users, categories or tabs with names that differ only in letter case | The second one gets a number after its name, for example `plex (2)`. A user with a changed name logs in with the new name |
| A record without a name | A name from its id, for example `Tab 12` or `user-21` |
| A user in a group that doesn't exist | The user is in the group for new users |
| A tab in a category that doesn't exist | The tab is in the category for new tabs |
| More than one group for new users, category for new tabs, or tab that opens first | Only the first one keeps the role |
| No group for new users, or no category for new tabs | One is picked. The report names it |
| No Admin group or no Guest group | The group is added |
| A second group with the level of another group | The group is left out. Its users are in the other group |
| A second category with the number of another category | The category gets the next free number |
| A tab with a level, a range of levels or a type that isn't possible | Only the Admin sees the tab, or the tab opens in a frame |
| A date that can't be read | The user has no date |

Dates of the old install have no time zone. The import reads them as UTC.

## 📄 The report

The report starts with `Dry run: nothing was written.` or `The import is complete.`, then the counts. A part without lines isn't printed.

| Part | What it holds |
|---|---|
| Records | The number of records of each kind, and the number of copied files. Then the number of sessions found, which aren't imported |
| Repairs | Records that the import changed. [Repairs](#-repairs) tells you why |
| Left out | Records that the import didn't store |
| To look at | Pictures that moved or are missing, and folders that the import didn't copy |
| Tables that this version does not import yet | The name and the number of rows of each table |
| Settings that this application does not know | The names of the settings in `legacy_settings` |
| Settings that this application does not need | The names of the settings that weren't stored |
| PHP files of the old install | Files that need a new home, because this version can't run them |

The report holds names of records and of settings. It holds no password, no token and no key. [Moving from PHP Organizr](moving-from-php.md#-2-import) shows a sample.

## 🔓 If an encrypted value can't be opened

PHP Organizr encrypts some values with the `organizrHash` setting of the install. If someone changed that setting later, the old values can't be opened, not even by PHP Organizr itself.

The import then leaves the setting empty, keeps the old value in `legacy_settings`, and writes a line in the report. Enter the value again on the settings page of the new install.
