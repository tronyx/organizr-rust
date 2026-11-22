#!/usr/bin/env bash
# Starts real Sonarr, Radarr, SABnzbd and qBittorrent in containers and asks them what
# the homepage asks them, with the client code of this application.
#
# The stand-ins of the tests are written from the code of PHP Organizr and from API
# documents. This script shows whether the real applications answer in the same way.
# It needs Docker, cargo and nothing else. Run it from the root of the repository.

set -euo pipefail

# The versions that the answers were last compared with.
SONARR=lscr.io/linuxserver/sonarr:4.0.20.3014-ls5
RADARR=lscr.io/linuxserver/radarr:7.0.3.10918-ls328
SABNZBD=lscr.io/linuxserver/sabnzbd:5.1.3-ls282
QBITTORRENT=lscr.io/linuxserver/qbittorrent:5.3.0_v2.1.2-ls487

# A made-up API key for Sonarr and Radarr, which take their key from the environment.
ARR_KEY=0123456789abcdef0123456789abcdef

NAMES="organizr-real-sonarr organizr-real-radarr organizr-real-sabnzbd organizr-real-qbittorrent"

stop_all() {
    # shellcheck disable=SC2086
    docker stop $NAMES >/dev/null 2>&1 || true
}
trap stop_all EXIT
stop_all

# Pulls an image, and tries again when the registry refuses, for example because of its
# rate limit: four tries, with 10, 20 and 40 seconds between them.
pull() {
    local wait=10
    for try in 1 2 3 4; do
        if docker pull --quiet "$1" >/dev/null; then
            return 0
        fi
        if [ "$try" -lt 4 ]; then
            echo "The pull of $1 failed; trying again in $wait seconds." >&2
            sleep "$wait"
            wait=$((wait * 2))
        fi
    done
    echo "The pull of $1 failed four times." >&2
    return 1
}

for image in "$SONARR" "$RADARR" "$SABNZBD" "$QBITTORRENT"; do
    pull "$image"
done

docker run --detach --rm --name organizr-real-sonarr \
    --env "SONARR__AUTH__APIKEY=$ARR_KEY" --env SONARR__AUTH__METHOD=External \
    --publish 127.0.0.1:58989:8989 "$SONARR" >/dev/null
docker run --detach --rm --name organizr-real-radarr \
    --env "RADARR__AUTH__APIKEY=$ARR_KEY" --env RADARR__AUTH__METHOD=External \
    --publish 127.0.0.1:57878:7878 "$RADARR" >/dev/null
docker run --detach --rm --name organizr-real-sabnzbd \
    --publish 127.0.0.1:58081:8080 "$SABNZBD" >/dev/null
# qBittorrent refuses a login when the port of the address is not its own port, so the
# port is the same inside and outside the container.
docker run --detach --rm --name organizr-real-qbittorrent \
    --env WEBUI_PORT=58080 --publish 127.0.0.1:58080:58080 "$QBITTORRENT" >/dev/null

# Waits until an address answers at all, for at most two minutes.
wait_for() {
    for _ in $(seq 1 60); do
        if curl --silent --max-time 2 --output /dev/null "$1"; then
            return 0
        fi
        sleep 2
    done
    echo "no answer from $1" >&2
    return 1
}
wait_for http://127.0.0.1:58989/ping
wait_for http://127.0.0.1:57878/ping
wait_for http://127.0.0.1:58081/
wait_for http://127.0.0.1:58080/

# SABnzbd makes its API key at the first start, and qBittorrent a password for the
# session. Neither is printed.
SABNZBD_KEY=""
QBITTORRENT_PASSWORD=""
for _ in $(seq 1 30); do
    SABNZBD_KEY=$(docker exec organizr-real-sabnzbd sh -c \
        'grep -E "^api_key" /config/sabnzbd.ini 2>/dev/null | cut -d" " -f3' || true)
    QBITTORRENT_PASSWORD=$(docker logs organizr-real-qbittorrent 2>&1 |
        grep -oE "session: [A-Za-z0-9]+" | tail -1 | cut -d" " -f2 || true)
    if [ -n "$SABNZBD_KEY" ] && [ -n "$QBITTORRENT_PASSWORD" ]; then
        break
    fi
    sleep 2
done
if [ -z "$SABNZBD_KEY" ] || [ -z "$QBITTORRENT_PASSWORD" ]; then
    echo "SABnzbd or qBittorrent did not make its key or password" >&2
    exit 1
fi

ORGANIZR_REAL_SETTINGS=$(
    cat <<EOF
{
  "sonarrURL": "http://127.0.0.1:58989",
  "sonarrToken": "$ARR_KEY",
  "radarrURL": "http://127.0.0.1:57878",
  "radarrToken": "$ARR_KEY",
  "sabnzbdURL": "http://127.0.0.1:58081",
  "sabnzbdToken": "$SABNZBD_KEY",
  "qBittorrentURL": "http://127.0.0.1:58080",
  "qBittorrentUsername": "admin",
  "qBittorrentPassword": "$QBITTORRENT_PASSWORD",
  "qBittorrentApiVersion": "2"
}
EOF
)
export ORGANIZR_REAL_SETTINGS

cargo test --locked --package organizr real_applications -- --nocapture
