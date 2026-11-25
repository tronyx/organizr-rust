# 🧪 Tests of the settings

[← All documentation](README.md)

Organizr has small tests that tell you whether a setting works before you depend on it: can this page show in a frame, does this folder exist, does this schedule make sense. This page lists them, with their buttons and routes. Only an Admin or a Co-Admin can run a test.

The tests for homepage services (the **Test Connection** buttons) are in [The homepage](homepage.md#-test-the-connection-of-a-service). To test all your services at once from the command line, see [Checking your services](checking-your-services.md).

## 🔘 The buttons

Most tests have a button in the same place as in PHP Organizr:

| Test | Button | Where | Before you save? |
|---|---|---|---|
| A page in a frame | **Test the tab** | Below the address in the dialog of a tab (**Settings › Tab Editor › Tabs**) | ✅ Uses the address in the dialog |
| A folder | **Test Backup Save Path** | Below **Backup Save Path** on **Settings › System Settings › Backup** | ✅ Uses the folder in the field |
| A cron schedule | **Test Cron Schedule** | Below each **Cron Schedule**: on **Settings › System Settings › Updates**, on **Settings › System Settings › Backup**, and in the section "Schedule" of the health checks plugin | ✅ Uses the schedule in the field |
| The scheduler | None | A script sends `GET /test/cron` | |
| The webhook for the logs | **Test Slack** | Section "Slack" of **Settings › System Settings › Logs** | ❌ Save first |
| The LDAP server | **Test Connection** and **Test Login** | Section "Sign-in method" of **Settings › System Settings › Sign-in**, when the authentication backend is LDAP. See [Logging in with LDAP, FTP, Emby or Jellyfin](login-services.md#-the-two-tests) | ❌ Save first |

## 🔌 The routes

Each test is a route below `/api/v2`. A script can call it with the API key.

| Test | Route | Body |
|---|---|---|
| Whether a page can show in a frame | `POST /test/iframe` | `{"url": "https://sonarr.example.com"}` |
| Whether a folder is there | `POST /test/folder` | `{"folder": "/backups"}` |
| Whether a cron schedule can be read | `POST /test/cron` | The schedule as one JSON string, for example `"*/5 * * * *"` |
| Whether the scheduler runs | `GET /test/cron` | None |
| The webhook for the logs | `POST /test/slack-logs` | None |
| The LDAP server: a bind with the bind account | `POST /test/ldap` | None |
| The LDAP server: a login | `POST /test/ldap/login` | `{"username": "robin", "password": "..."}` |

## 🪟 A page in a frame

The test fetches the page and reads the header `X-Frame-Options` of the answer:

| Header | Result |
|---|---|
| No header | The page can show in a frame |
| `DENY` | The page can't show in a frame |
| `SAMEORIGIN` | The page can show only when it's on the host of Organizr |
| `ALLOW-FROM` with addresses | The page can show when one address names the host of Organizr |

The answer is `200` with "URL approved for iFrame", or `409` with "URL failed approval for iFrame". A page that can't be reached fails the test.

> [!NOTE]
> The test doesn't read the header `Content-Security-Policy`, which can forbid frames too. A page that does that passes the test but stays blank in its tab. Set that tab to open in a new window.

## 📁 A folder

The test passes when the folder is there. It doesn't make the folder (PHP Organizr does).

Use it for the folder of the [backups](backups.md): Organizr uses that folder only when it exists.

## ⏰ A cron schedule and the scheduler

- `POST /test/cron` reads a schedule the way the scheduler reads it. [Scheduled jobs](scheduled-jobs.md) describes the form. The answer is `200` with "Schedule was validated", or `500` with what's wrong.
- `GET /test/cron` answers `200` when the scheduler woke up in the last two minutes. The scheduler is part of the server, so this fails only in the first moments after a start, or when the scheduler stopped.

## 💬 The webhook for the logs

The test sends one message to the webhook in `slackLogWebhook`. It uses these settings:

| Setting | Key |
|---|---|
| Send Logs to Slack | `sendLogsToSlack` |
| Slack Webhook URL | `slackLogWebhook` |
| Slack Channel for Webhook | `slackLogWebHookChannel` |

- A Slack webhook needs the channel. A Discord webhook needs none.
- For a Discord webhook, the message has Discord's form, or Slack's form when the address ends with `/slack`.
- The webhook address is a secret. No answer and no log line names it.

This version of Organizr doesn't send its log to the webhook yet. The test only tells you that the webhook takes a message.

## 🧙 Tests of PHP's set-up wizard

PHP Organizr has two more tests for its set-up wizard. They have nothing to test here, and each has one fixed answer:

| Route | Answer | Why |
|---|---|---|
| `POST /test/path` | `401` with "Endpoint disabled as database already exists" | The database is there before the wizard starts, and the wizard asks for no folder |
| `POST /test/database` | `409` with "Database connection test not available for driver: " and the driver | You choose the database before the first start. [Choosing a database](choosing-a-database.md) tells you how |
