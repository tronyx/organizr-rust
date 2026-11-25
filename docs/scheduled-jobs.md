# Scheduled jobs

Organizr runs its scheduled jobs itself, inside the server. PHP Organizr needs an entry in the cron table of the system that calls `cron.php` each minute. This application needs no such entry.

## The jobs

| Job | Switch | Schedule |
|---|---|---|
| Run of the [health checks plugin](health-checks-plugin.md) | `HEALTHCHECKS-cron-run-enabled` | `HEALTHCHECKS-cron-run-schedule` |
| [Backup](backups.md) of the install | `autoBackupCronEnabled` | `autoBackupCronSchedule` |
| Version check: is a newer release of Organizr out? (D-23 in [deviations](deviations.md)) | `autoUpdateCronEnabled` | `autoUpdateCronSchedule` |

More jobs come with the features that need them.

## How the scheduler works

The server looks at the schedules at each full minute. It starts each job whose switch is on and whose schedule names that minute.

- A change of a switch or of a schedule counts from the next minute on. You do not restart the server.
- A job that still runs is not started a second time.
- A job that runs longer than ten minutes is stopped.
- A schedule that is not valid starts nothing. The log names the job and the schedule.
- The jobs stop with the server.

## The form of a schedule

A schedule has five fields with spaces between them, as in a cron table:

| Field | Values |
|---|---|
| Minute | 0 to 59 |
| Hour | 0 to 23 |
| Day of the month | 1 to 31 |
| Month | 1 to 12, or `jan` to `dec` |
| Day of the week | 0 to 7, or `sun` to `sat`. 0 and 7 are both Sunday |

A field can be:

| Form | Meaning | Example |
|---|---|---|
| `*` | Every value | `*` |
| A number | That value | `30` |
| A range | The values from the first to the last | `8-18` |
| A step | Every n-th value | `*/5`, `10-30/5` |
| A list | Each of its parts | `0,30` |

Examples:

| Schedule | Meaning |
|---|---|
| `*/5 * * * *` | Every five minutes |
| `0 * * * *` | At each full hour |
| `30 4 * * *` | Each day at 04:30 |
| `0 8 * * mon-fri` | On each working day at 08:00 |

A schedule can also be one of these words:

| Word | Meaning |
|---|---|
| `@hourly` | At each full hour |
| `@daily` or `@midnight` | Each day at 00:00 |
| `@weekly` | Each Sunday at 00:00 |
| `@monthly` | On the first day of each month at 00:00 |
| `@yearly` or `@annually` | On 1 January at 00:00 |

If a schedule limits the day of the month and the day of the week, a day that fits one of the two is enough. `0 12 1 * mon` is noon on the first day of each month and on each Monday.

The times are in the time zone of the server. Set the environment variable `TZ` of the container to your time zone, for example `TZ=Europe/Berlin`.

## Test a schedule

The settings of the updates, of the backups and of the health checks plugin have a button **Test Cron Schedule** below the schedule. A script can ask the same question, and it can also ask whether the scheduler runs. [Tests of the settings](settings-tests.md) describes the two routes.
