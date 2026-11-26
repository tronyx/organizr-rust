# 🧦 API SOCKS

[← All documentation](README.md)

The API SOCKS lets a script reach the API of Sonarr, Radarr and a few others **through Organizr**. Organizr passes the request on to the application's address from the settings, so the application needs no port forward and no reverse proxy of its own. You need this page if a script or a mobile app should talk to your apps from outside.

The name comes from PHP Organizr. It has nothing to do with the SOCKS proxy protocol.

## 📱 The applications

| Application | Name in the path | Where the script puts the application's key |
|---|---|---|
| Sonarr | `sonarr` | The header `X-Api-Key` |
| Radarr | `radarr` | The header `X-Api-Key` |
| Lidarr | `lidarr` | The header `X-Api-Key` |
| SABnzbd | `sabnzbd` | The query field `apikey` |
| NZBGet | `nzbget` | The header `Authorization` with the user name and password, or the path: `/<user>:<password>/jsonrpc` |
| Tautulli | `tautulli` | The query field `apikey` |
| qBittorrent | `qbittorrent` | No key. qBittorrent must let Organizr in without a login, for example with "Bypass authentication for clients in whitelisted IP subnets" |

**Organizr never adds the key from its own settings.** The script brings the key, as with PHP Organizr. So a caller can do only what their own key allows.

## 🚀 Switch it on

1. Open **Settings › Tab Editor › Homepage Items**, and select the picture of the application.
2. In the application's address field, enter its **full address with the host**, for example `http://192.168.1.20:8989`.
3. In the group **API SOCKS**, switch on **Enable**. It's off at the start.
4. Set **Minimum Authentication** to the group with the least access that may use it. It starts at the guest level, which means **everybody, also people who aren't logged in**. Pick a level with more access unless the application checks a key itself.
5. Select **Save**.

> [!IMPORTANT]
> **Use a full address.** PHP Organizr puts the scheme and host of the incoming request in front of an address that is only a path, such as `/sonarr`. That host comes from the caller's `Host` header, so anybody could make Organizr send requests to any host they like. Organizr Rust refuses such an address with "Use the full address of the service" (D-137). And pick a level above guest unless the application checks a key itself.

## 🚨 Watch out

- **NZBGet and a proxy password.** Organizr passes the caller's `Authorization` header on to NZBGet, as PHP Organizr does. A browser behind a reverse proxy that asks for a password sends that password in this header. With the API SOCKS of NZBGet on, that password reaches NZBGet.
- **Applications that trust Organizr.** An application that lets requests from Organizr's address in without a key (such as qBittorrent with "Bypass authentication for clients in whitelisted IP subnets") lets in everybody who has the level of its API SOCKS, for every method, DELETE included. Choose the level with care.

## 🔗 The address of a request

With one address in the settings:

```text
https://organizr.example.com/api/v2/socks/sonarr/api/v3/queue?page=2
```

Organizr sends `GET http://192.168.1.20:8989/api/v3/queue?page=2` to Sonarr. Everything after `/socks/sonarr` (the path and the query) goes after Sonarr's address.

When the address field holds several addresses separated by commas, name the one you want by its position, starting at 1:

```text
https://organizr.example.com/api/v2/multiple/socks/sonarr/2/api/v3/queue
```

The form must match the settings:

- With commas, `/socks/` gets `409 "Multiple URLs found in field, please use /api/v2/multiple/socks endpoint"`.
- Without commas, `/multiple/socks/` gets `409 "Multiple endpoint accessed but multiple URLs not found in field, please use /api/v2/socks endpoint"`.

## 📤 What goes to the application

| Part | What Organizr passes on |
|---|---|
| **Method** | GET, POST, PUT and DELETE as they are. PATCH and OPTIONS go as GET, as in PHP Organizr |
| **Body** | A POST with a JSON or form body, a PUT with a JSON body, at most 2 MiB. Other bodies, and the body of a DELETE, are dropped |
| **Headers** | Only the application's key header and `Content-Type`. Cookies and the Organizr `token` header never leave Organizr. Only NZBGet gets the `Authorization` header, as its key |
| **Query** | Every field, except `apikey` when it holds the Organizr API key |

The path can't leave the application's address. A part `.` or `..`, or an encoded form such as `%2e%2e` or `%2f`, gets `400 "The path must not leave the address of the application"`.

## 📥 The answer

The answer holds the application's response body as it is. Its status is always `200` and its content type is JSON, as in PHP Organizr. A script reads the body to see whether the application accepted the request.

| Status | Text | Meaning |
|---|---|---|
| `404` | `Application not supported for socks` | The name in the path isn't one of the seven |
| `409` | `SOCKS module is not enabled` | The application's **Enable** switch is off |
| `401` | `Not Authorized` | The caller doesn't have the level |
| `409` | `Instance not found in field` | The position in the multiple form names no address |
| `500` | A text without the address | The application can't be reached, or doesn't answer within 120 seconds |

The log never holds the address of a request or the body of an answer, because both can contain keys.
