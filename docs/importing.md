# Importing an install of PHP Organizr

The `organizr import` command moves an install of PHP Organizr into this application. It reads the old install and writes a new one. It never changes the old install, and it never runs PHP code.

You run the import one time, before the first start of the new server. The import needs a new, empty database.

## What the import needs

- The folder of the old install. This is the folder of the PHP application, which holds `index.php` and the folder `data`. For a container image with a `/config` volume, you can give the volume folder. The import then looks in `www/organizr`, `www/Dashboard`, `www` and `html`.
- The SQLite database file of the old install. The configuration file of the old install names the file by its path on the old server. The import looks for the file at that path, and at the end of that path below the folder that you gave. If the import does not find the file, name the file with the option `--database`.
- A data folder for the new install, which is empty or does not exist.

This version imports an old install that uses SQLite. It refuses an old install that uses MySQL or PostgreSQL. The new install can use SQLite or PostgreSQL. [Choosing a database](choosing-a-database.md) tells you how to select one.

## Import the old install

1. Stop the old Organizr, or make a copy of its folder and its database file. A database that changes during the import can give an incomplete result.
2. Do a dry run. A dry run reads the old install, prints the report, and writes nothing.

   ```sh
   organizr import --from /path/to/old/organizr --dry-run
   ```

3. Read the report. The section [The report](#the-report) tells you what each part means.
4. Run the import. Use the same data folder, database and secret key options that you will use for `organizr serve`.

   ```sh
   organizr import --from /path/to/old/organizr --data-dir /config
   ```

5. Keep the report. It lists what you must look at after the import.
6. Make a backup of the file `secret.key` in the data folder. [Secrets](secrets.md) tells you why the database is not readable without that file.
7. Start the server with `organizr serve` and log in.

With the container image, run the import in a container that has the old install as a second, read-only volume:

```sh
docker run --rm \
  -v organizr-data:/config \
  -v /path/to/old/config:/old:ro \
  ghcr.io/tronyx/organizr-rust:develop \
  import --from /old --data-dir /config
```

If the import fails, it stores no records. Correct the cause and run the import again. Files that a failed import copied stay in the data folder, and the next import writes over them.

If the new database holds records, the import stops and changes nothing. To import again, start with a new data folder or a new, empty database.

## What is imported

| From the old install | In the new install |
|---|---|
| Groups, users, categories, tabs | The same records, with the same ids |
| Bookmarks and bookmark categories of the bookmark plugin | The same records, with the same ids |
| Passwords of users | The same hashes. Each user logs in with the old password |
| Second factor of a user, Plex token of a user | Kept, sealed with the key of the new install |
| Settings in `data/config/config.php` | The settings of the new install. A setting that has its default value is not stored |
| Values that PHP encrypted (keys that hold the word "password") | Opened with the `organizrHash` of the old install and sealed with the key of the new install |
| API key | Kept, sealed. Scripts that use the key continue to work |
| Registration password | Kept as a hash |
| Uploaded pictures in `data/userTabs` | Copied to `userTabs` in the data folder |
| Pictures that a person added to `plugins/images` | Copied to `userTabs` in the data folder. See [Pictures](#pictures) |
| Custom favicon files in `data/favicon` | Copied to `favicon` in the data folder |

## What is not imported

- Sessions. Every user logs in again.
- The tables of the invites plugin and of the chat: `invites`, `chatroom` and `options`. This version does not have these features yet. The report gives the number of rows of each table. The rows stay in the old database, so keep that file.
- The picture cache in `data/cache`. The application makes its cache again.
- PHP files that a person added in `data/pages`, `data/routes` and `data/plugins`. The report lists them. This application cannot run PHP.
- The settings `driver`, `dbHost`, `dbUsername`, `dbPassword`, `dbLocation`, `dbName`, `logLocation`, `configVersion` and `organizrHash`. The new install gets its database from `ORGANIZR_DATABASE_URL`, writes its log to the standard output, and seals secrets with its own key.

## Settings that the application does not know

An old configuration file can hold settings that this application does not know. Examples are the settings of a plugin that is not part of Organizr, and settings of another Organizr version. The import keeps each such setting in the table `legacy_settings`, sealed with the key of the new install. The report lists their names. Nothing reads these settings at this time.

## Pictures

The PHP application has picture folders below `plugins/images`. People put pictures of their own into those folders. This application ships only the pictures of Organizr itself. So the import copies these pictures to the uploaded pictures:

- Each picture below `plugins/images` that a group, a user, a category, a tab or a setting uses, if the application does not ship the picture.
- Each other picture in `plugins/images/tabs` that the application does not ship. The picture chooser then continues to show them.

A record that used such a picture then points to the new place, `data/userTabs/<name>`. If an uploaded picture has the same name, the copied picture gets the name of its old folder in front, for example `tabs-logo.png`.

The import copies GIF, PNG, JPEG and SVG files. The report names each picture that a record uses and that the import cannot copy.

## Repairs

The PHP application checks less than this application. Its database can hold records that this application cannot hold. The import repairs each such record and writes one line about it in the report.

| In the old install | In the new install |
|---|---|
| Two groups, users, categories or tabs with names that differ only in letter case | The second one gets a number after its name, for example `plex (2)`. A user with a changed name logs in with the new name |
| A record without a name | A name from its id, for example `Tab 12` or `user-21` |
| A user in a group that does not exist | The user is in the group of new users |
| A tab in a category that does not exist | The tab is in the category of new tabs |
| More than one group of new users, category of new tabs, or tab that opens first | Only the first one keeps the role |
| No group of new users, or no category of new tabs | One is selected. The report names it |
| No Admin group or no Guest group | The group is added |
| A second group with the level of another group | The group is left out. Its users are in the other group |
| A second category with the number of another category | The category gets the next free number |
| A tab with a level, a range of levels or a type that is not possible | Only the Admin sees the tab, or the tab opens in a frame |
| A date that cannot be read | The user has no date |

Dates of the old install have no time zone. The import reads them as UTC.

## The report

The report has these parts. A part without lines is not printed.

| Part | What it holds |
|---|---|
| Records | The number of records of each kind, and the number of copied files |
| Repairs | Records that the import changed. The section [Repairs](#repairs) tells you why |
| Left out | Records that the import did not store |
| To look at | Pictures that moved or that are missing, and folders that the import did not copy |
| Tables that this version does not import yet | The name and the number of rows of each table |
| Settings that this application does not know | The names of the settings in `legacy_settings` |
| Settings that this application does not need | The names of the settings that were not stored |
| PHP files of the old install | Files that need a new home, because this application cannot run them |

The report holds names of records and of settings. It holds no password, no token and no key.

## If an encrypted value cannot be opened

The PHP application encrypts some values with the `organizrHash` setting of the install. If a person changed that setting later, the old values cannot be opened. The PHP application cannot open them either.

The import then leaves the setting empty, keeps the old value in `legacy_settings`, and writes a line in the report. Enter the value again on the settings page of the new install.
