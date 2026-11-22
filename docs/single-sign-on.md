# Single sign-on

With single sign-on, a person who logs in to Organizr is logged in to your other applications too: Plex Web, Ombi, Overseerr and Tautulli.

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

### Fallback users

Ombi and Overseerr can have a fallback user: `ombiFallbackUser` with `ombiFallbackPassword`, and `overseerrFallbackUser` with `overseerrFallbackPassword`. A user whom the application does not know is then logged in as the fallback user. Without a fallback user, such a user gets no session of that application.

For Overseerr, the fallback user is an e-mail address.

### Tautulli

`tautulliURL` can hold more than one address, with commas between them. Each Tautulli install gets its own cookie, at the path of the install.

`ssoTautulliAuth` is a level. Only users of that level, or with more access, get a session of Tautulli.

## Good to know

- An application that does not answer does not stop the login. Organizr waits for each application for at most one minute, divided by the number of applications that are switched on.
- Organizr checks the certificate of an application that it reaches by a name over HTTPS. It does not check the certificate of an application that it reaches by an IP address over HTTPS, because no authority signs a certificate for such an address.
- The Plex cookie holds the Plex token of the user, and scripts of the pages on your domain can read it. Plex Web needs that. Switch `ssoPlex` on only when you trust every application on the domain.

## Not built yet

PHP Organizr also has single sign-on for Jellyfin, Petio and Komga. This application does not have these yet. The groups of Petio and Komga on the page **SSO** say so at their top.
