# Single sign-on

With single sign-on, a person who logs in to Organizr is logged in to your other applications too: Plex Web, Ombi, Overseerr, Tautulli and Jellyfin.

## How it works

At a login, Organizr asks each application for a session of the user. It hands each session to the browser as the cookie that the application itself sets after a login. When the person opens the application, the browser sends the cookie, and the application sees a user who is logged in.

For this, the application and Organizr must be on the same domain, for example `organizr.example.com` and `ombi.example.com`. The cookies are for the whole domain, `.example.com`. An application that is reached by another domain, or by an IP address, does not get them.

Organizr asks with what the login brought:

- After a login through Plex, with the Plex token of the user. See [Logging in with a Plex account](plex-login.md).
- After a login with a password of Organizr, with the user name, or the e-mail address, and that password. This works when the user has the same name and password in the application.

A logout of Organizr makes the browser drop the cookies.

## Set it up

1. Log in as an Admin.
2. Open **Settings**, then **System Settings**, then **SSO**.
3. For each application, select its group, switch **Enable** on, and enter its address.
4. Select **Save**.
5. Log out and log in again. The sessions are made at a login.

| Application | Switch | Address | Cookie |
|---|---|---|---|
| Plex Web | `ssoPlex` | | `mpt`, the Plex token of the user |
| Ombi | `ssoOmbi` | `ombiURL` | `Auth` |
| Overseerr | `ssoOverseerr` | `overseerrURL` | `connect.sid` |
| Tautulli | `ssoTautulli` | `tautulliURL` | `tautulli_token_<id of the install>` |
| Jellyfin | `ssoJellyfin` | `jellyfinURL` and `jellyfinSSOURL` | `jellyfin_credentials` and `user-<user>-<server>`. See [Jellyfin](#jellyfin) |

### Fallback users

Ombi and Overseerr can have a fallback user: `ombiFallbackUser` with `ombiFallbackPassword`, and `overseerrFallbackUser` with `overseerrFallbackPassword`. A user whom the application does not know is then logged in as the fallback user. Without a fallback user, such a user gets no session of that application.

For Overseerr, the fallback user is an e-mail address.

### Tautulli

`tautulliURL` can hold more than one address, with commas between them. Each Tautulli install gets its own cookie, at the path of the install.

`ssoTautulliAuth` is a level. Only users of that level, or with more access, get a session of Tautulli.

### Jellyfin

Jellyfin keeps its login in the storage of the browser, not in a cookie. So Organizr does two things:

1. At a login with a password, it asks Jellyfin for a session with the user name and the password. It hands the session to the browser as two cookies, as PHP Organizr does: `jellyfin_credentials`, with the address of Jellyfin, the user and the access token, and `user-<user>-<server>`, with the user.
2. The page of Organizr copies the two cookies into the storage of the browser. Jellyfin reads them there and shows the user as logged in.

The storage of the browser belongs to one site. So the copy of step 2 works only when Jellyfin is a path of the site of Organizr, for example `https://organizr.example.com/jellyfin`. The settings:

| Setting | Meaning |
|---|---|
| **Jellyfin API URL** (`jellyfinURL`) | The address that Organizr asks, for example `http://jellyfin:8096` |
| **Jellyfin SSO URL** (`jellyfinSSOURL`) | The address of Jellyfin in the browser, for example `https://organizr.example.com/jellyfin` |

In Jellyfin, enter the path as the **Base URL** in **Dashboard**, **Networking**. The [Reverse proxy](reverse-proxy.md#a-tab-with-a-path) page tells you how to send a path to an application.

#### Jellyfin on a site of its own

When Jellyfin has a site of its own, for example `https://jellyfin.example.com`, the page of Organizr cannot write into its storage. A script on the pages of Jellyfin then does the copy. With nginx, add this to the location of Jellyfin:

```nginx
location / {
    proxy_pass http://jellyfin:8096;
    proxy_set_header Host $host;
    # The page must come uncompressed, so that nginx can add the script.
    proxy_set_header Accept-Encoding "";
    sub_filter '</head>' '<script>(function(){var c=document.cookie.split("; ");for(var i=0;i<c.length;i++){var at=c[i].indexOf("="),n=c[i].slice(0,at);if(n==="jellyfin_credentials"||n.indexOf("user-")===0){try{localStorage.setItem(n,decodeURIComponent(c[i].slice(at+1)));}catch(e){}}}})();</script></head>';
    sub_filter_once on;
}
```

The script holds no `$`: nginx reads `$` in `sub_filter` as the start of a variable. Set **Jellyfin SSO URL** to the address of that site. Organizr and Jellyfin must share the domain, as for every application of single sign-on.

#### What to know about Jellyfin

- A login through Plex or OpenID Connect brings no password, so it gets no session of Jellyfin. The user logs in to Jellyfin as usual.
- A logout of Organizr drops the cookies, and the page of Organizr takes the login of Jellyfin out of the storage of the browser.
- With Jellyfin on a site of its own, the page of Organizr cannot reach the storage of that site. After a logout of Organizr, Jellyfin stays logged in until the person logs out in Jellyfin. And while the cookies are there, the script copies the login again, also after a logout in Jellyfin.
- The access token of Jellyfin is in a cookie that scripts of the pages on your domain can read, as in PHP Organizr, and Jellyfin keeps it in the storage of the browser itself.

## Good to know

- An application that does not answer does not stop the login. Organizr waits for each application for at most one minute, divided by the number of applications that are switched on.
- Organizr checks the certificate of an application that it reaches by a name over HTTPS. It does not check the certificate of an application that it reaches by an IP address over HTTPS, because no authority signs a certificate for such an address.
- The Plex cookie holds the Plex token of the user, and scripts of the pages on your domain can read it. Plex Web needs that. Switch `ssoPlex` on only when you trust every application on the domain.

## Not built yet

PHP Organizr also has single sign-on for Petio and Komga. This application does not have these yet. Their groups on the page **SSO** say so at their top.
