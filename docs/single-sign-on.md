# 🔗 Single sign-on

[← All documentation](README.md)

Log in to Organizr once, and you're logged in to your other applications too: **Plex Web, Ombi, Overseerr, Tautulli, Petio, Komga and Jellyfin**. No second login screen when you open a tab. You need this page if your users keep asking "why do I have to log in again?"

## 🧠 How it works

When someone logs in to Organizr, Organizr asks each application for a session for that user. It hands each session to the browser as the same cookie the application would set after its own login. When the person opens the application, the browser sends the cookie, and the application sees a logged-in user.

What Organizr uses to ask:

- **After a Plex login**, the user's Plex token. See [Logging in with a Plex account](plex-login.md).
- **After a login with an Organizr password**, the user name (or e-mail address) and that password. This works when the user has the same name and password in the application.

A logout of Organizr makes the browser drop the cookies.

### 🌍 Everything on one domain

The cookies are set for the whole domain, for example `.example.com`. So the applications and Organizr must share it: `organizr.example.com`, `ombi.example.com`, and so on. An application reached by another domain, or by an IP address, doesn't get the cookies.

> [!WARNING]
> **Every site under that domain receives the cookies**, including the Organizr login cookie, whether it's one of your apps or not. A compromised or untrusted app on a sibling subdomain can read every visitor's Organizr session. Put only applications you trust under the same domain as Organizr.

## 🚀 Set it up

1. Log in as an Admin.
2. Open **Settings › System Settings › SSO**. Each application has its own section.
3. For each application you want, fill in its section and switch on **Enable**:

   | Section | What to fill in |
   |---|---|
   | **Plex** | **API Key/Token**, **Plex Machine ID** and **Plex Admin Username or Email** (the same settings as the [Plex login](plex-login.md#-set-it-up)) |
   | **Tautulli** | **Multiple URL's** (one address or several) and **Minimum Authentication** |
   | **Overseerr** | **URL**, **API Key/Token**, and optionally a [fallback user](#-fallback-users) |
   | **Petio** | **URL**, **API Key/Token**, and optionally a [fallback user](#-fallback-users) |
   | **Ombi** | **URL**, **API Key/Token**, and optionally a [fallback user](#-fallback-users) |
   | **Jellyfin** | **Jellyfin API URL** and **Jellyfin SSO URL** (see [Jellyfin](#-jellyfin)) |
   | **Komga** | **URL**, **Minimum Authentication**, and the [Komga](#-komga) passwords |

4. Select **Save**.
5. Log out and log in again. Sessions are only made at a login.

### 📋 The switches and cookies

| Application | Switch | Address setting | Cookie it gets |
|---|---|---|---|
| Plex Web | `ssoPlex` | | `mpt`, the user's Plex token |
| Ombi | `ssoOmbi` | `ombiURL` | `Auth` |
| Overseerr | `ssoOverseerr` | `overseerrURL` | `connect.sid` |
| Tautulli | `ssoTautulli` | `tautulliURL` | `tautulli_token_<id of the install>` |
| Petio | `ssoPetio` | `petioURL` | `petio_jwt` |
| Komga | `ssoKomga` | `komgaURL` | `komga_token`. See [Komga](#-komga) |
| Jellyfin | `ssoJellyfin` | `jellyfinURL` and `jellyfinSSOURL` | `jellyfin_credentials` and `user-<user>-<server>`. See [Jellyfin](#-jellyfin) |

> [!CAUTION]
> **The Plex cookie holds the user's Plex token, and scripts on any page of your domain can read it.** Plex Web needs it that way. Switch on SSO for Plex only when you trust every application on the domain.

## 🛟 Fallback users

Ombi, Overseerr and Petio can have a fallback user: `ombiFallbackUser` with `ombiFallbackPassword`, `overseerrFallbackUser` with `overseerrFallbackPassword`, and `petioFallbackUser` with `petioFallbackPassword`. (The form labels them **… Fallback Email** and **… Fallback Password**.) A user the application doesn't know is then logged in as the fallback user. Without a fallback user, such a user simply gets no session for that application.

For Overseerr, the fallback user is an e-mail address.

> [!WARNING]
> **Everyone the application doesn't know gets the rights of the fallback account**: guests who signed up with the registration password included. Make it an account with the fewest rights, never an admin of the application.

## 📥 Ombi imports its users

When someone's first login through Plex or Emby creates their Organizr account, Organizr asks Ombi to import the users of that media server, as PHP Organizr does. So Ombi knows the new person, and SSO to Ombi works for them right away.

This happens when **Authentication Backend** (`authBackend`) is `plex` or one of the Emby backends, and Ombi's **URL** (`ombiURL`) and **API Key/Token** (`ombiToken`) are set. Organizr doesn't wait for Ombi: the login goes on, and the log says whether Ombi ran the import (D-124).

## 📊 Tautulli

- **Multiple URL's** (`tautulliURL`) can hold more than one address, with commas between them. Each Tautulli install gets its own cookie, at the path of the install.
- **Minimum Authentication** (`ssoTautulliAuth`) is a level. Only users of that level, or with more access, get a Tautulli session.

## 📚 Komga

Komga knows its users by their e-mail address. Organizr asks Komga for a session with the user's e-mail address and the password of the login.

| Setting | What it does |
|---|---|
| **Minimum Authentication** (`ssoKomgaAuth`) | A level. Only users of that level, or with more access, get a Komga session |
| **Komga Master Password** (`komgaSSOMasterPassword`) | The password Organizr uses for a login that brings none (through Plex or OpenID Connect) |
| **Komga Fallback Email**, **Komga Fallback Password** (`komgaFallbackUser`, `komgaFallbackPassword`) | The fallback account: an e-mail address and its password |

> [!WARNING]
> With **Komga Master Password** set, every Plex or OpenID Connect login gets a Komga session through it. Like the fallback user, give it as few rights as you can.

A logout of Organizr also ends the session at Komga. PHP Organizr calls an address that current Komga versions no longer have, so there the session lives on (D-117).

## 🪼 Jellyfin

Jellyfin keeps its login in the browser's storage, not in a cookie. So Organizr does two things:

1. At a login with a password, it asks Jellyfin for a session with the user name and password. It hands the session to the browser as two cookies, as PHP Organizr does: `jellyfin_credentials` (the Jellyfin address, the user and the access token) and `user-<user>-<server>` (the user).
2. The Organizr page copies the two cookies into the browser's storage. Jellyfin reads them there and shows the user as logged in.

Browser storage belongs to one site. So step 2 only works when Jellyfin is a **path** of the Organizr site, for example `https://organizr.example.com/jellyfin`.

| Setting | What it does |
|---|---|
| **Jellyfin API URL** (`jellyfinURL`) | The address Organizr calls, for example `http://jellyfin:8096` |
| **Jellyfin SSO URL** (`jellyfinSSOURL`) | The address of Jellyfin in the browser, for example `https://organizr.example.com/jellyfin` |

In Jellyfin, enter the path as the **Base URL** in **Dashboard › Networking**. [A tab with a path](reverse-proxy.md#-a-tab-with-a-path) shows how to send a path to an application.

### 🏡 Jellyfin on a site of its own

When Jellyfin has its own site, for example `https://jellyfin.example.com`, the Organizr page can't write into its storage. A small script on the Jellyfin pages does the copy instead. With nginx, add this to the Jellyfin location:

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

The script contains no `$` on purpose: nginx reads `$` in `sub_filter` as the start of a variable. Set **Jellyfin SSO URL** to the address of that site. Organizr and Jellyfin must still share the domain.

### 💡 Good to know about Jellyfin

- A login through Plex or OpenID Connect brings no password, so it gets no Jellyfin session. The user logs in to Jellyfin as usual.
- A logout of Organizr drops the cookies, and the Organizr page removes the Jellyfin login from the browser's storage.
- With Jellyfin on its own site, the Organizr page can't reach that site's storage. After a logout of Organizr, Jellyfin stays logged in until the person logs out in Jellyfin. And while the cookies are there, the script copies the login back, even after a logout in Jellyfin.
- The Jellyfin access token is in a cookie that scripts on your domain's pages can read, as in PHP Organizr, and Jellyfin keeps it in the browser's storage itself.

## 🔧 Good to know

- **A slow app doesn't block the login.** Organizr waits for each application at most one minute, divided by the number of applications switched on.
- **Certificates:** Organizr checks the certificate of an application it reaches by a **name** over HTTPS. It does **not** check the certificate of an application it reaches by an **IP address** over HTTPS, because no authority signs certificates for addresses. Use names if the connection crosses a network you don't trust.
