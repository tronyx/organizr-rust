# The log

Organizr writes its log to two places:

- The standard output of the server. With Docker, `docker logs` shows it.
- One file for each day, in the folder `logs` of the data folder. PHP Organizr keeps its log in the same way.

## The log files

The file of a day has the name `organizr-<date>.log`, for example `organizr-2026-11-20.log`. The date and the times in the file are in UTC. Each line of the file is one record, as a JSON object:

```json
{"log_level":"INFO","message":"a user logged in","channel":"Login","username":"robin","trace_id":"0b1f6a3e-7c1d-4a52-9d55-3f2a8c9e1b47","request_id":"6f1c2b9e-4d0a-4f3e-8a71-2c5d9e0b3a64","file":"crates/api/src/handlers/login.rs","line":117,"context":{},"remote_ip_address":"192.0.2.7","datetime":"2026-11-20 03:15:00.123456","timezone":"UTC"}
```

| Field | What it holds |
|---|---|
| `log_level` | The level: `DEBUG`, `INFO`, `WARNING` or `ERROR` |
| `message` | What happened |
| `channel` | The part of the server that wrote the record. The page of the log calls it the function |
| `username` | The user the record is about, or `System` |
| `trace_id` | A random id of the record. The page of the log finds a record by it |
| `request_id` | The id of the request that wrote the record, or `null` for a record that no request wrote, for example at the start of the server |
| `file`, `line` | The place in the source code that wrote the record |
| `context` | The other values of the record, for example the reason of an error |
| `remote_ip_address` | The address of the client, when the record names one. Otherwise `null` |
| `datetime`, `timezone` | The time of the record, and `UTC` |

To read the files without the web interface, use a tool for JSON lines, for example `jq`:

```sh
jq -r '[.datetime, .log_level, .message] | @tsv' data/logs/organizr-2026-11-20.log
```

## Find the lines of a request

Each answer of the server has the header `X-Request-Id`: a random id that the server makes for the request. Every line that the request writes to the log carries the same id: `request_id` in the log file, and `request{request_id=...}` in the standard output. The server never takes the id from a header of the client.

To find the lines of a request, read the header of its answer, for example in the network tools of the browser or with `curl -i`. Then search the log for the id:

```sh
grep '"request_id":"6f1c2b9e-4d0a-4f3e-8a71-2c5d9e0b3a64"' data/logs/organizr-*.log
docker logs organizr 2>&1 | grep 'request_id=6f1c2b9e-4d0a-4f3e-8a71-2c5d9e0b3a64'
```

The page of the log in the web interface shows the request id in the details of a record.

## What the file holds

The file holds the records of Organizr itself. The records of the libraries below, for example of the database driver, go to the standard output only. A record never holds a password, a key or a token, and never an address of another service that can hold one. A record can hold the name of a user and the address of a client. On an install that is not set up yet, a record holds the set-up code of the wizard (see the README). The code is new at each start and works only until the set-up is done.

A backup holds the log files, because they are in the data folder.

## Settings

Open **Settings**, and select the tab **Logs**.

| Setting | Key | Default |
|---|---|---|
| Lowest level in the log file | `logLevel` | `INFO` |
| Number of log files to keep | `maxLogFiles` | 7 |
| Time between two loads of the live log | `logLiveUpdateRefresh` | 5 seconds |
| Records on a page of the log | `logPageSize` | 50 |

`logLevel` has the eight levels of PHP Organizr. Organizr writes records of four of them: `DEBUG`, `INFO`, `WARNING` and `ERROR`. A level above `ERROR`, for example `CRITICAL`, keeps every record out of the file. When there are more files than `maxLogFiles`, Organizr deletes the oldest ones. A change of either setting counts from the next record on. The server does not need a restart.

The standard output has a level of its own: the environment variable `ORGANIZR_LOG`, for example `debug`. The default is `info`. `logLevel` does not change the standard output, and `ORGANIZR_LOG` does not change the file.

PHP Organizr has the setting `logLocation` for another folder. Organizr does not use it: the log files are always in the folder `logs` of the data folder.

When the server cannot write to the folder, it says so once on its standard error, and the log goes on in the standard output.

## Read the log in the web interface

An Admin or a Co-Admin can read the log on the page **Settings** > **Logs**, below the settings:

- **File** chooses the file of a day, or **All** for all of them.
- **Level** shows the records of one level only.
- **Reload** loads the page again. **Live update** loads it again and again, in the time of `logLiveUpdateRefresh`.
- **Details** shows the trace id, the request id, the place in the source code and the context of a record.
- **Purge this file** deletes the chosen file. The server makes a new file with the next record of the day.

## The API

The routes of PHP Organizr are kept. Both need the level of a Co-Admin.

| Route | What it does |
|---|---|
| `GET /api/v2/log[/{number}[/{trace_id}]]` | A page of the records, the newest first. `number` is the place of a file on the list of files, the newest first, from 0. `all` or `combined-logs` reads all files. The query takes `pageSize` (default 1000), `offset` (default 0) and `filter` (a level, or `NONE` for all, the default). With a `trace_id`, the answer is the record with that id |
| `DELETE /api/v2/log[/{number}]` | Deletes a file. Without `number`, the newest file |

[Deviations](deviations.md) lists the differences from PHP Organizr in row D-84.
