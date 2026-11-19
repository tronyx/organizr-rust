# Forward auth

With forward auth, the login of Organizr protects your other applications. A reverse proxy asks Organizr about each request for an application. Organizr answers "yes" or "no" from the session cookie of the browser. The application itself needs no login.

A reverse proxy is the web server in front of your applications, for example nginx, Traefik or Caddy.

## What you need

- The other application and Organizr are on the same domain, for example `sonarr.example.com` and `organizr.example.com`. The login cookie of Organizr is for the whole domain, so the browser sends it to both.
- The reverse proxy is listed in `ORGANIZR_TRUSTED_PROXIES`. Organizr believes the forwarding headers of a listed proxy only. Without the entry, Organizr sees the address of the proxy as the address of every caller.

## The question

The proxy sends a request to one of these routes. Every HTTP method asks the same question.

| Route | Meaning |
|---|---|
| `/api/v2/auth/{group}` | The caller needs the level `{group}` or more access |
| `/api/v2/auth-{group}` | The same, in a short form |
| `/api/v2/organizr-auth/{group}` | The same route under another name |
| `/api/v2/auth/{group}/{type}/{ips}` | The same, with a list of addresses |

`{group}` is the level of a group: `0` for Admin, `1` for Co-Admin, `4` for User in a new install, `999` for everybody. A user passes when the level of the user is that number or a smaller one. So `/api/v2/auth/4` lets Users, Power Users, Super Users, Co-Admins and Admins through.

`{group}` can also be the name of a tab. The question is then about the group of that tab. If no tab has the name, the question is about the Admin.

Without a `{group}`, the question is about the Admin.

### Lists of addresses

`{type}` and `{ips}` add a list of addresses to the question. `{ips}` holds addresses and host names, which commas separate.

| `{type}` | Meaning |
|---|---|
| `allow`, `whitelist`, `white`, `wl`, `w` | A caller from one of the addresses passes, whatever the group |
| `deny`, `blacklist`, `black`, `bl`, `b` | A caller from one of the addresses does not pass |

The short form takes the lists from the query: `whitelist` and `blacklist`, or `type` and `ips`. For example `/api/v2/auth-1?whitelist=192.168.1.10`.

An entry of a list is one address. A network such as `192.168.1.0/24` is not an entry.

### The group in the query

A query that holds `group=` asks in the short form, and everything after `group=` is the group. So `/api/v2/auth?group=1` and `/api/v2/?group=1` are the same question as `/api/v2/auth-1`. Old proxy configurations use this form.

## The answer

| Status | Meaning |
|---|---|
| `200` | The caller passes |
| `401` | The caller does not pass |
| `302` | The caller does not pass, and the browser must go to the login page. See [Traefik](#traefik) |

A `200` answer for a caller who passed by the group has three headers. The proxy can hand them to the application.

| Header | Value |
|---|---|
| `X-Organizr-User` | The name of the user, or `Guest` |
| `X-Organizr-Email` | The e-mail address of the user |
| `X-Organizr-Group` | The level of the user |

A locked user does not pass. A caller who is not logged in is a guest, with the level `999`.

## nginx

nginx asks with the `auth_request` module. This example protects one application for the level 1.

```nginx
# In the server block of the application.
location / {
    auth_request /organizr-auth;
    # A caller who does not pass goes to the login page of Organizr.
    error_page 401 =302 https://organizr.example.com/?return=$scheme://$host$request_uri;
    proxy_pass http://sonarr:8989;
}

location = /organizr-auth {
    internal;
    proxy_pass http://organizr:8080/api/v2/auth/1;
    proxy_pass_request_body off;
    proxy_set_header Content-Length "";
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

After the login, Organizr sends the browser back to the address in `return`.

## Traefik

Traefik asks with the `forwardAuth` middleware.

```yaml
http:
  middlewares:
    organizr-users:
      forwardAuth:
        address: "http://organizr:8080/api/v2/auth/4"
        authResponseHeaders:
          - "X-Organizr-User"
          - "X-Organizr-Email"
          - "X-Organizr-Group"
```

Traefik hands the answer of Organizr to the browser when the caller does not pass. So Organizr can send the browser to its login page itself. It does that in two cases:

- The setting `traefikAuthEnable` is on.
- The request has the header `X-Forwarded-Server: traefik`. Traefik writes its host name into that header, so this applies when the host name of the Traefik container is `traefik`.

The answer is then a redirect to Organizr, with the address that the caller wanted in `return`. The setting `traefikDomainOverride` names the address of Organizr for this redirect, for example `https://organizr.example.com/`. Without it, Organizr uses the address of the request.

## Caddy

Caddy asks with the `forward_auth` directive.

```caddyfile
sonarr.example.com {
    forward_auth organizr:8080 {
        uri /api/v2/auth/1
        copy_headers X-Organizr-User X-Organizr-Email X-Organizr-Group
    }
    reverse_proxy sonarr:8989
}
```

## After the login

A person who was sent to the login page comes with the wanted address in `return`. After the login, Organizr sends the browser there.

Organizr follows such an address only when it belongs to this site: the host of Organizr itself, or a host that shares the login cookie with it. A link with the address of another site in `return` leads nowhere. PHP Organizr follows every address.

## What differs from PHP Organizr

- PHP Organizr believes the forwarding headers of every sender. This application believes a proxy in `ORGANIZR_TRUSTED_PROXIES` only. So a caller cannot claim an address of an allow list, and cannot make Organizr write another address into the redirect to the login page.
- In PHP Organizr, `group=` in the query of every API route makes the request a forward-auth question. Here it does that for the forward-auth routes and for `/api/v2/` only.
- The name of a tab is compared as text. In PHP Organizr, `%` and `_` in the name stand for other characters.
- A host name in a list stands for all its addresses, with IPv6. In PHP Organizr it stands for its first IPv4 address.

[Deviations](deviations.md) has the full entry, D-59.
