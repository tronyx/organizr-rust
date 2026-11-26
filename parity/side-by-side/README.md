# Side by side on a copy of a live install

The parity tests compare PHP Organizr and this version on made-up data. This folder does the same on a copy of a real install: it imports the copy into this version, starts PHP Organizr 2.1.5000 on the same copy, and asks both the same routes.

It's a check for a move, not a test that CI runs. Everything runs on a Docker network without access to the outside, and nothing listens on your machine. So the copied install sends no requests to Plex, mail servers or other services.

> [!CAUTION]
> The copy holds the password hashes, API key and service secrets of the install. Keep it in a folder that only you can read, and delete it, with the volumes, when you're done.

## What you need

- Docker with Compose.
- A copy of the `/config` folder of the PHP install: the folder that holds `www/organizr` and `www/db`. PHP writes to the copy while it runs, so never point this at the live folder.

## Steps

Run these in this folder. `OLD_COPY` is the copy; `PG_PASSWORD` is any password for the throwaway database.

```bash
export OLD_COPY=/path/to/copy/config
export PG_PASSWORD=any-throwaway-password

# 1. Import the copy, first as a dry run. Do this before PHP starts on the copy.
docker compose run --rm import import --from /old --dry-run
docker compose run --rm import

# 2. Start both servers.
docker compose up --build --detach rust php

# 3. Compare. Give routes to compare only those.
docker compose run --rm compare
docker compose run --rm compare python -I /compare.py /api/v2/tabs /api/v2/users

# 4. Throw everything away.
docker compose --profile import --profile compare down --volumes
```

## Reading the output

Each line names the caller (`guest`, or `apikey` for the API key of the old install), the route, and the status of each side. Then it counts the JSON paths that are the same, that differ, and that only one side has, and names the first ones.

The script prints paths and types, never values. A path under **differ** with the same type on both sides has another value.

Most differences are approved ones. [The deviations](../../docs/deviations.md) list each, with its reason. Look for a difference that no deviation explains.

## What it doesn't check

- Logins with real passwords, the browser, and the reverse proxy.
- Routes that change something: the script sends GET requests only.
- Homepage items and other routes that need the services of the install, which the closed network can't reach.
