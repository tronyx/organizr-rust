# Backups

Organizr can make a backup of an install, and it can restore one. A backup is one zip file.

## What a backup holds

| Part of the install | In the backup |
|---|---|
| The files of the data folder: uploaded pictures, favicon files and other files | Yes |
| The SQLite database | Yes, as a copy that is whole also while the server runs |
| A PostgreSQL database | No. Save it with the tools of PostgreSQL, for example `pg_dump` |
| The key for the stored secrets, the file `secret.key` | No, never |
| The picture cache and older backups | No |

A backup does not hold the key, so a person who gets a backup cannot read the passwords and tokens of your other services. They are sealed in the database. Without the key, a restored install cannot read them either. Keep a copy of the file `secret.key` at a safe place that is not the place of your backups. [Secrets](secrets.md) tells you more about the key.

A backup holds the password hashes of all users. Only an Admin or a Co-Admin can make, save and delete backups, as in PHP Organizr. Keep this in mind when you make somebody a Co-Admin.

## Make a backup

1. Log in as an Admin or a Co-Admin.
2. Open **Settings** > **System Settings** > **Backup**.
3. Select **Make a backup now**. The list shows the new backup.
4. To copy a backup to your computer, select **Save** in its row.

The backups are in the folder `backups` of the data folder. To use another folder, enter it in the field **Backup Save Path**, on the tab **Cron** of **Settings** > **System Settings** > **Main**. The folder must exist, and the user of the server must be able to write to it. Select **Test Backup Save Path** to see whether the folder exists.

## Backups on a schedule

These settings are on the tab **Cron** of **Settings** > **System Settings** > **Main**.

| Setting | Key | Default |
|---|---|---|
| Auto-Backup Organizr | `autoBackupCronEnabled` | Off |
| Cron Schedule | `autoBackupCronSchedule` | `@weekly` |
| # Backups Keep | `keepBackupsCountCron` | 20 |
| Backup Save Path | `backupLocation` | Empty: the folder `backups` in the data folder |

After each new backup, Organizr deletes the oldest backups over the number to keep. The number 0 keeps every backup. [Scheduled jobs](scheduled-jobs.md) tells you how to write a schedule. Select **Test Cron Schedule** to see whether the scheduler can read it. [Tests of the settings](settings-tests.md) tells you how a script checks that the scheduler runs.

## Restore a backup

The command `organizr restore` puts a backup back. It writes the files into the data folder. With SQLite, it also writes the database.

1. Stop the server.
2. Make sure that the file `secret.key` of the install that made the backup is in the data folder.
3. Run the command. With the container image:

   ```sh
   docker run --rm \
     -v organizr-data:/config \
     -v /path/to/backups:/backups:ro \
     ghcr.io/tronyx/organizr-rust:develop \
     restore --from "/backups/backup[2026-11-20_03-15 - ab][0.1.0].zip" --overwrite
   ```

4. Start the server.

| Option | Meaning |
|---|---|
| `--from` | The backup |
| `--overwrite` | Replace the database of the data folder. Without this option, the command stops when a database exists, and changes nothing |
| `--data-dir` | The data folder, when it is not the default |

A file of the data folder that is also in the backup is replaced. A file that is only in the data folder stays.

A backup of an older version of Organizr works with a newer version: the server brings the database up to date when it starts. A backup of a newer version does not work with an older version.

### With PostgreSQL

The command restores the files only, and says so. Restore the database with the tools of PostgreSQL, from a save that you made at the time of the backup.

## The routes

| Route | Purpose |
|---|---|
| `GET /api/v2/backup` | The list of the backups, the newest first |
| `POST /api/v2/backup` | Make a backup |
| `GET /api/v2/backup/{name}` | Send a backup as a file |
| `DELETE /api/v2/backup/{name}` | Delete a backup |

All four are for an Admin, or for a script with the API key.
