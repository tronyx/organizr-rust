# API SOCKS

With the API SOCKS, a script reaches the API of an application through Organizr. Organizr passes the request on to the address of the application in the settings. So the application needs no port forward and no reverse proxy of its own.

The name comes from PHP Organizr. It has nothing to do with the SOCKS protocol of proxies.

## The applications

| Application | Address in the path | Where the script puts the key of the application |
|---|---|---|
| Sonarr | `sonarr` | The header `X-Api-Key` |
| Radarr | `radarr` | The header `X-Api-Key` |
| Lidarr | `lidarr` | The header `X-Api-Key` |
| SABnzbd | `sabnzbd` | The query field `apikey` |
| NZBGet | `nzbget` | The header `Authorization` with the user name and the password, or the path: `/<user>:<password>/jsonrpc` |
| Tautulli | `tautulli` | The query field `apikey` |
| qBittorrent | `qbittorrent` | No key. qBittorrent must let Organizr in without a login, for example with "Bypass authentication for clients in whitelisted IP subnets" |

Organizr never adds the key from its own settings. The script brings the key, as with PHP Organizr. So a person who can use the API SOCKS can do only what the key of that person allows.

## Switching it on

Open **Settings**, then **Tab Editor**, then **Homepage Items**, and select the picture of the application. Each of the seven applications has two settings in the group **API SOCKS**:

- **Enable** switches the API SOCKS of the application on. It is off at the start.
- **Minimum Authentication** names the group with the least access that may use it. At the start it is the guest level, so everybody may use it once it is on. Choose a level with more access unless the application asks for a key itself.

Use a full address with its host, such as `http://192.168.1.20:8989`. An address that is only a path, such as `/sonarr`, works as in PHP Organizr: Organizr puts the scheme and the host of the request in front. The host then comes from the `Host` header of the caller, so a caller can choose where such a request goes.

## Warnings

- **NZBGet and a password of a reverse proxy.** Organizr passes the `Authorization` header of the caller on to NZBGet, as PHP Organizr does. A browser behind a reverse proxy that asks for a password sends that password in this header. With the API SOCKS of NZBGet on, that password then reaches NZBGet.
- **Addresses that are only a path.** An address such as `/sonarr` takes its host from the request, so the caller chooses the host. Enter the full address of the application instead.
- **Applications that trust Organizr.** An application that lets requests from the address of Organizr in without a key, such as qBittorrent with "Bypass authentication for clients in whitelisted IP subnets", lets in everybody who has the level of its API SOCKS, for every method, also DELETE. Choose the level with care.

## The address of a request

For one address in the settings:

```
https://organizr.example.com/api/v2/socks/sonarr/api/v3/queue?page=2
```

Organizr sends `GET http://192.168.1.20:8989/api/v3/queue?page=2` to Sonarr. Everything after `/socks/sonarr` goes after the address of Sonarr: the path, and the query.

When the address field holds several addresses with commas between them, name the place of the address, from 1:

```
https://organizr.example.com/api/v2/multiple/socks/sonarr/2/api/v3/queue
```

The form must match the settings. With commas, `/socks/` gets `409 "Multiple URLs found in field, please use /api/v2/multiple/socks endpoint"`. Without commas, `/multiple/socks/` gets `409 "Multiple endpoint accessed but multiple URLs not found in field, please use /api/v2/socks endpoint"`.

## What goes to the application

- **Method**: GET, POST, PUT and DELETE go as they are. PATCH and OPTIONS go as GET, as in PHP Organizr.
- **Body**: a POST takes a body of JSON or of a form, a PUT one of JSON, at most 2 MiB. Other bodies, and the body of a DELETE, do not go.
- **Headers**: only the key header of the application and `Content-Type`. Cookies and the `token` header of Organizr never leave Organizr. Only NZBGet gets the `Authorization` header, as its key.
- **Query**: every field of the query, except `apikey` when it holds the API key of Organizr.

The path cannot leave the address of the application. A part `.` or `..`, or a code such as `%2e%2e` or `%2f`, gets `400 "The path must not leave the address of the application"`.

## The answer

The answer holds the body of the answer of the application, as it is. Its status is always 200 and its content type is JSON, as in PHP Organizr. A script reads the body to see whether the application accepted the request.

| Status | Text | Meaning |
|---|---|---|
| `404` | `Application not supported for socks` | The name in the path is not one of the seven |
| `409` | `SOCKS module is not enabled` | The switch of the application is off |
| `401` | `Not Authorized` | The caller does not have the level |
| `409` | `Instance not found in field` | The place in the multiple form names no address |
| `500` | A text without the address | The application cannot be reached, or does not answer within 120 seconds |

The log never holds the address of a request or the body of an answer, because both can hold keys.
