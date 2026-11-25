# ⏰ Scheduled jobs

[← All documentation](README.md)

Organizr runs its scheduled jobs itself, inside the server. This page lists the jobs and shows how to write a schedule. PHP Organizr needs a system cron entry that calls `cron.php` every minute; here you need nothing of the sort.

## 📋 The jobs

| Job | Switch | Schedule | Where to set it |
|---|---|---|---|
| Run the [health checks plugin](health-checks-plugin.md) | `HEALTHCHECKS-cron-run-enabled` | `HEALTHCHECKS-cron-run-schedule` | The plugin's settings, section "Schedule" |
| [Back up](backups.md#-automatic-backups) the install | `autoBackupCronEnabled` | `autoBackupCronSchedule` | **Settings › System Settings › Backup** |
| Check for a newer release of Organizr (D-23 in [Deviations](deviations.md)) | `autoUpdateCronEnabled` | `autoUpdateCronSchedule` | **Settings › System Settings › Updates** |

More jobs come with the features that need them.

## 🔧 How the scheduler works

The server looks at the schedules at each full minute. It starts every job whose switch is on and whose schedule names that minute.

- A change of a switch or a schedule counts from the next minute on. No restart needed.
- A job that is still running isn't started a second time.
- A job that runs longer than ten minutes is stopped.
- A schedule that isn't valid starts nothing. The log names the job and the schedule.
- The jobs stop with the server.

## 🌍 Time zone

The times are in the time zone of the server, which is UTC unless you say otherwise. Set the environment variable `TZ` of the container to your time zone:

```yaml
services:
  organizr:
    environment:
      TZ: Europe/Berlin
```

Then run `docker compose up -d` to apply it.

## 📝 The form of a schedule

A schedule has five fields with spaces between them, as in a cron table:

| Field | Values |
|---|---|
| Minute | 0 to 59 |
| Hour | 0 to 23 |
| Day of the month | 1 to 31 |
| Month | 1 to 12, or `jan` to `dec` |
| Day of the week | 0 to 7, or `sun` to `sat`. 0 and 7 are both Sunday |

Each field can be:

| Form | Meaning | Example |
|---|---|---|
| `*` | Every value | `*` |
| A number | That value | `30` |
| A range | The values from the first to the last | `8-18` |
| A step | Every n-th value | `*/5`, `10-30/5` |
| A list | Each of its parts | `0,30` |

### 💡 Examples

| Schedule | Meaning |
|---|---|
| `*/5 * * * *` | Every five minutes |
| `0 * * * *` | At each full hour |
| `30 4 * * *` | Each day at 04:30 |
| `0 8 * * mon-fri` | Each working day at 08:00 |

### 🔖 Shortcuts

| Word | Meaning |
|---|---|
| `@hourly` | At each full hour |
| `@daily` or `@midnight` | Each day at 00:00 |
| `@weekly` | Each Sunday at 00:00 |
| `@monthly` | On the first day of each month at 00:00 |
| `@yearly` or `@annually` | On January 1 at 00:00 |

> [!NOTE]
> If a schedule limits both the day of the month and the day of the week, a day that fits either one is enough. `0 12 1 * mon` is noon on the first of each month **and** on every Monday.

## 🧪 Test a schedule

The settings of the updates, the backups and the health checks plugin have a **Test Cron Schedule** button below the schedule. It checks the schedule in the field, even before you save.

A script can ask the same question, and it can also ask whether the scheduler runs. [Tests of the settings](settings-tests.md#-a-cron-schedule-and-the-scheduler) describes the two routes.
