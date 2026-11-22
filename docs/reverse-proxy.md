# Reverse proxy

A reverse proxy is the web server in front of your applications, for example nginx, Traefik or Caddy. It gives Organizr an address such as `https://organizr.example.com`, with HTTPS. This page tells you how to put Organizr behind one, and how to make the tabs of Organizr work through it.

To protect your other applications with the login of Organizr, read [Forward auth](forward-auth.md) too.

## What Organizr needs from the proxy

The proxy must send three things with each request:

| Header | Why |
|---|---|
| `Host` | The name of Organizr that the browser used. Organizr completes the addresses of tabs with it. |
| `X-Forwarded-Proto` | `https` when the browser used HTTPS. Organizr then makes `https://` addresses and secure cookies. |
| `X-Forwarded-For` | The address of the browser. Organizr uses it for the log, for the local network rules and for forward auth. |

Organizr believes `X-Forwarded-Proto` and `X-Forwarded-For` only from a proxy that you name in the environment variable `ORGANIZR_TRUSTED_PROXIES`. So a caller cannot claim another address.

## ORGANIZR_TRUSTED_PROXIES

The variable holds the addresses and networks of your proxies, separated by commas:

```yaml
services:
  organizr:
    environment:
      ORGANIZR_TRUSTED_PROXIES: "172.18.0.0/16"
```

When the proxy is a container on the same Docker network as Organizr, use the network of that Docker network. This command shows it:

```sh
docker network inspect <network> --format '{{(index .IPAM.Config 0).Subnet}}'
```

When the proxy runs on another computer, use the address of that computer.

Without the variable, Organizr believes no proxy. Then:

- the tabs and the links of Organizr start with `http://`, also when the browser uses HTTPS;
- the log shows the address of the proxy for every caller;
- every caller seems to come from the address of the proxy, so the local network rules apply to nobody or to everybody.

Organizr writes this warning to its log once when a request comes with `X-Forwarded-For` from an address that is not in the list:

```text
a request with an X-Forwarded-For header came from an address that is not a trusted proxy, so the header was ignored. If this address is your reverse proxy, add it to ORGANIZR_TRUSTED_PROXIES.
```

## nginx

```nginx
server {
    listen 443 ssl;
    server_name organizr.example.com;
    # ssl_certificate and ssl_certificate_key of your certificate.

    location / {
        proxy_pass http://organizr:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # The chat plugin keeps this request open and sends each new message at once.
    location = /api/v2/plugins/chat/events {
        proxy_pass http://organizr:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_buffering off;
        proxy_read_timeout 1h;
    }
}
```

Do not leave out `proxy_set_header Host $host`. Without it, nginx sends the name of the upstream, `organizr:8080`, and the tabs get addresses such as `http://organizr:8080/sonarr/`.

## Traefik

Traefik sends `Host`, `X-Forwarded-Proto` and `X-Forwarded-For` by itself. Labels for the container of Organizr:

```yaml
services:
  organizr:
    labels:
      - traefik.enable=true
      - traefik.http.routers.organizr.rule=Host(`organizr.example.com`)
      - traefik.http.routers.organizr.entrypoints=websecure
      - traefik.http.routers.organizr.tls=true
      - traefik.http.services.organizr.loadbalancer.server.port=8080
```

## Caddy

Caddy sends the three headers by itself:

```caddyfile
organizr.example.com {
    reverse_proxy organizr:8080
}
```

## Tabs

A tab opens an application in a frame of Organizr, or in a new browser tab. The address of a tab can have two forms:

| Address of the tab | What the browser opens |
|---|---|
| A full address, for example `https://sonarr.example.com/` | That address |
| A path, for example `/sonarr/` | The path on the host of Organizr: `https://organizr.example.com/sonarr/` |

### A tab with a path

The proxy of Organizr must send the path to the application. Without such a location, the proxy sends the request to Organizr, and the tab shows Organizr inside Organizr.

For nginx, add a location for each application to the server block of Organizr. This example also protects Sonarr with the login of Organizr, for the level 1 (Co-Admin):

```nginx
location /sonarr/ {
    auth_request /organizr-auth-1;
    proxy_pass http://sonarr:8989;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}

# One such location for each level that your tabs use.
location = /organizr-auth-1 {
    internal;
    proxy_pass http://organizr:8080/api/v2/auth/1;
    proxy_pass_request_body off;
    proxy_set_header Content-Length "";
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

The application must know that it runs under the path. In Sonarr, Radarr and the other *arr applications, enter `/sonarr` in **Settings**, **General**, **URL Base**.

[Forward auth](forward-auth.md) tells you what the levels mean, and how to do the same with Traefik and Caddy.

### A tab in a frame

A browser shows an application in a frame only when both of these are true:

- The application allows it. Some applications send the header `X-Frame-Options` or a `Content-Security-Policy` with `frame-ancestors`, which forbid frames. In nginx, `proxy_hide_header X-Frame-Options;` in the location of the application removes the first one. Otherwise, set the tab to open in a new browser tab.
- The address uses HTTPS when Organizr does. A page with HTTPS cannot show an `http://` address in a frame. Use a path, or give the application an address with HTTPS.

Organizr itself does not let other sites show it in a frame.
