# Tests of the settings

Organizr has tests that tell you whether a value of the settings works, before you depend on it. Only an Admin or a Co-Admin can run a test. The tests of the services of the homepage are in [The homepage](homepage.md#test-the-connection-of-a-service).

Each test is a route below `/api/v2`. A script can send the request with the API key. The settings pages have a button for most tests, in the place where PHP Organizr has it:

| Test | Button |
|---|---|
| A page in a frame | **Test the tab**, below the address in the dialog of a tab. The test uses the address in the dialog, also before you save. |
| A folder | **Test Backup Save Path**, below the field **Backup Save Path** in the group "Cron" of **System Settings**, **Main**. The test uses the folder in the field, also before you save. |
| A cron schedule | **Test Cron Schedule**, below each field **Cron Schedule**: in the group "Cron" of **System Settings**, **Main**, and in the group "Cron" of the health checks plugin. The test uses the schedule in the field, also before you save. |
| The scheduler | No button. A script sends `GET /test/cron`. |
| The webhook for the logs | **Test Slack**, in the group "Logs" of **System Settings**, **Main**. Save the settings before you test. |
| The LDAP server | **Test Connection** and **Test Login**, in the group "Authentication" of **System Settings**, **Main**, when the authentication backend is LDAP. Save the settings before you test. See [Logging in with LDAP, FTP, Emby or Jellyfin](login-services.md#the-two-tests). |

The routes and their bodies:

| Test | Route | Body |
|---|---|---|
| Whether a page can show in a frame | `POST /test/iframe` | `{"url": "https://sonarr.example.com"}` |
| Whether a folder is there | `POST /test/folder` | `{"folder": "/backups"}` |
| Whether a cron schedule can be read | `POST /test/cron` | The schedule as one JSON text, for example `"*/5 * * * *"` |
| Whether the scheduler runs | `GET /test/cron` | None |
| The webhook for the logs | `POST /test/slack-logs` | None |
| The LDAP server: a bind with the bind account | `POST /test/ldap` | None |
| The LDAP server: a login | `POST /test/ldap/login` | `{"username": "robin", "password": "..."}` |

## A page in a frame

The test asks for the page and reads the header `X-Frame-Options` of the answer.

| Header | Result |
|---|---|
| No header | The page can show in a frame. |
| `DENY` | The page cannot show in a frame. |
| `SAMEORIGIN` | The page can show only when it is on the host of Organizr. |
| `ALLOW-FROM` with addresses | The page can show when one address names the host of Organizr. |

The answer is `200` with "URL approved for iFrame", or `409` with "URL failed approval for iFrame". A page that cannot be reached fails the test.

The test does not read the header `Content-Security-Policy`. A page can forbid frames with that header too. Such a page stays empty in a tab. Set its tab to open in a new window.

## A folder

The test passes when the folder is there. It makes no folder. PHP Organizr makes the folder when it is missing.

Use the test for the folder of the [backups](backups.md). Organizr uses that folder only when it is there.

## A cron schedule and the scheduler

`POST /test/cron` reads a schedule as the scheduler reads it. [Scheduled jobs](scheduled-jobs.md) describes the form. The answer is `200` with "Schedule was validated", or `500` with what is wrong.

`GET /test/cron` answers `200` when the scheduler woke in the last two minutes. The scheduler is a part of the server, so the test fails only in the first moments after a start, or when the scheduler stopped.

## The webhook for the logs

The test sends one message to the webhook of the setting `slackLogWebhook`. It needs these settings:

| Setting | Key |
|---|---|
| Switch | `sendLogsToSlack` |
| Address of the webhook | `slackLogWebhook` |
| Channel | `slackLogWebHookChannel` |

A webhook of Slack needs the channel. A webhook of Discord needs none. For a webhook of Discord, the message has the form of Discord, or the form of Slack when the address ends with `/slack`.

The address of a webhook is a secret. No answer and no log line names it.

This version of Organizr does not send its log to the webhook. The test tells you only that the webhook takes a message.

## Tests of the set-up of PHP Organizr

PHP Organizr has two more tests for its set-up wizard. They have nothing to test here, and each one has one fixed answer.

| Route | Answer | Reason |
|---|---|---|
| `POST /test/path` | `401` with "Endpoint disabled as database already exists" | The database is there before the wizard starts, and the wizard asks for no folder. |
| `POST /test/database` | `409` with "Database connection test not available for driver: " and the driver | You choose the database before the first start. [Choosing a database](choosing-a-database.md) tells you how. |
