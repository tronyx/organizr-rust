# 🗝️ Logging in with LDAP, FTP, Emby or Jellyfin

[← All documentation](README.md)

Let people log in to Organizr with the name and password they already have on another server of yours: an LDAP directory (Active Directory, OpenLDAP, FreeIPA), an FTP server, an Emby server, Emby Connect, or a Jellyfin server. For Plex, see [Logging in with a Plex account](plex-login.md).

## 🚀 Set it up

> [!WARNING]
> **Don't pick "Backend Only" until an account of the service can log in as an Admin.** With **Backend Only**, Organizr checks none of the passwords it stores. If the service is down, nobody can log in, not even the Admin from the set-up wizard. **Organizr DB + Backend** keeps the Admin's own password working.

1. Log in as an Admin.
2. Open **Settings › System Settings › Sign-in**. The section **Sign-in method** opens.
3. Set **Authentication Type**:

   | Choice | Which passwords work |
   |---|---|
   | **Organizr DB** | Only the passwords that Organizr stores. The settings of the services stay hidden |
   | **Organizr DB + Backend** | The service first. When the service says no, or can't be reached, the password that Organizr stores. **Recommended** |
   | **Backend Only** | Only the service. See the warning above |

4. Set **Authentication Backend** to your service. Its settings appear below.
5. Fill in the settings of the service (see [LDAP](#-ldap), [FTP](#-ftp) or [Emby and Jellyfin](#-emby-and-jellyfin)), and select **Save**.
6. For LDAP, use the two test buttons to check it (see [The two tests](#-the-two-tests)).
7. Log out, and log in with an account of the service.

| **Authentication Backend** | `authBackend` | Who checks the password |
|---|---|---|
| Ldap | `ldap` | Your LDAP server, by a bind with the user's name |
| Ftp | `ftp` | Your FTP server, by a login |
| Emby Local | `emby_local` | Your Emby server, by a login |
| Jellyfin | `jellyfin` | Your Jellyfin server, by a login |
| Emby Connect | `emby_connect` | Emby Connect (`connect.emby.media`), for an account that is linked to a user of your Emby server |
| Emby All | `emby_all` | Your Emby server first, then Emby Connect when the server says no |

## 👤 Who gets which account

When the service says yes, Organizr picks the Organizr account like this:

1. **The name.** For every service except the three Emby ones, the name is first changed to lower case, as in PHP Organizr.
2. **Find the account.** Organizr looks for an account with that user name, or that e-mail address, in any letter case.
3. **Or make one.** If there isn't one, Organizr makes an account in the group for new users:
   - The name is what the person typed. For Emby Connect, it's the name of the Emby Connect account.
   - The e-mail address is the `mail` of the LDAP entry, or the address of the Emby Connect account. Other services get a stand-in address, as in PHP Organizr.
4. **The password.** The password of the login becomes the password of the Organizr account. So the person can still log in when the service is down, if **Authentication Type** allows Organizr passwords.
5. **The second factor.** An account with a [second factor](second-factor.md) asks for its code, as in every login.

> [!IMPORTANT]
> **Admin and Co-Admin accounts are protected differently per service.**
> - **Emby Connect never opens an Admin or Co-Admin account.** Anyone can create an Emby Connect account with any free name, so such a login fails.
> - **LDAP, FTP and your own Emby or Jellyfin server can open any account, the Admin's too**, when the name or e-mail address matches. You run those servers, so you decide who has which name there. Make sure nobody on the service can have the name or e-mail address of your Organizr Admin, or give the Admin a name the service doesn't have.
> - **No service ever replaces the password of an Admin or Co-Admin account.** It stays the one set in Organizr.

After someone changes their password on the service, **the old one keeps working in Organizr** (with **Organizr DB + Backend**) until they log in once with the new one.

### 🧱 Failed logins

- A login that the service refuses counts as a failed login. After **Max Login Attempts** (**Settings › System Settings › Login & Security**, section **Lockout and sessions**), the client address and the name are locked out for a while, whatever the service says.
- A login with an empty password is refused without asking the LDAP or FTP server. Many servers treat an empty password as an anonymous login.

> [!NOTE]
> **Settings › System Settings › Sign-in** also has an **Auth proxy** section (**Auth Proxy**, **Auth Proxy Whitelist**, **Auth Proxy Header Name**, …). It's kept from PHP Organizr: Organizr stores these settings, but this version doesn't log anyone in from a header. To put a login in front of your apps, use [Forward auth](forward-auth.md); to log in through Authentik and similar, use [OpenID Connect](openid-connect.md).

## 📇 LDAP

| Setting | Key | Example |
|---|---|---|
| **Host Address** | `authBackendHost` | `ldap://ldap.example.com:389` |
| **Host Base DN** | `authBaseDN` | `dc=example,dc=com` |
| **Account Prefix** | `authBackendHostPrefix` | `uid=` for OpenLDAP, `EXAMPLE\` for Active Directory |
| **Account Suffix** | `authBackendHostSuffix` | `,ou=people,dc=example,dc=com` for OpenLDAP, `@example.com` for Active Directory |
| **Bind Username** | `ldapBindUsername` | `cn=organizr,dc=example,dc=com` |
| **Bind Password** | `ldapBindPassword` | Stored encrypted, like every secret |
| **LDAP Backend Type** | `ldapType` | `1` Active Directory, `2` OpenLDAP, `3` Free IPA |
| **Enable LDAP SSL** | `ldapSSL` | On for `ldaps` |
| **Enable LDAP TLS** | `ldapTLS` | On for StartTLS |

**Several servers.** **Host Address** can name more than one server, with commas between them. Organizr tries them in order and uses the first one that answers. Every server is asked on the port of the first address.

**The port.** Without a port, it's 389 for `ldap://` (and for an address without a scheme), and 636 for any other scheme.

**TLS.** Whether TLS is used depends on the switches, not the scheme, as in PHP Organizr. With TLS, Organizr checks the server certificate against the certificate authorities of the system. A server with a certificate from your own authority needs that authority installed in the system that runs Organizr (the container). PHP Organizr accepts any certificate.

> [!CAUTION]
> With both TLS switches off, the password of every login travels over the network in plain text. Use **Enable LDAP SSL** or **Enable LDAP TLS** unless Organizr and the LDAP server share a network that nobody else uses.

**LDAP Backend Type** is kept for PHP Organizr. Organizr sends the same requests for all three types.

### 🔍 What a login does

Each step has 5 seconds:

1. Organizr binds with **Bind Username** and **Bind Password**. Without a bind user name, it binds anonymously.
2. Organizr binds with prefix + user name + suffix, and the password of the login. If this bind works, the password is right.
3. Organizr binds with the bind account again, and reads the `mail` of the entry from step 2. If there's no such entry (for example with the `@example.com` form of Active Directory), the new account gets no address from LDAP.

A user name that contains a character with a special meaning in an LDAP name or filter is refused: `, + " \ < > ; = * ( )`, a control character, a space at the start or end, or `#` at the start. So a user name can't change which entry Organizr asks for.

### 🧪 The two tests

**Save the settings first:** both tests use the stored settings. Only an Admin or a Co-Admin can run them.

| Button | Route | Body | Answer |
|---|---|---|---|
| **Test Connection** | `POST /test/ldap` | None | `200 "LDAP connection successful"`, `409` with the error of the LDAP server, or `422 "authBaseDN and/or BackendHost not supplied"` |
| **Test Login** | `POST /test/ldap/login` | `{"username": "robin", "password": "..."}` | `200 "LDAP connection successful"`, `401 "Username/Password Failed to authenticate"`, `422 "Username and/or Password not supplied"`, or `500` with the error of the LDAP server |

**Test Login** does steps 1 and 2 of a login. The user name and password you type for the test aren't stored.

## 📂 FTP

| Setting | Key | Example |
|---|---|---|
| **Host Address** | `authBackendHost` | `ftps://ftp.example.com:21` |

- The address names one server. Without a port, the port is 21.
- Without a scheme, Organizr uses FTPS (`AUTH TLS` before the login), as PHP Organizr does when PHP has TLS. Any scheme other than `ftp` or `ftps` refuses every login.
- Organizr sends `USER` and `PASS`, and `QUIT` at the end. The password is right when the server answers `230`. The whole login has 20 seconds.
- With FTPS, the server certificate is checked, as for LDAP. A server with a self-signed certificate needs a certificate that the system trusts, or the plain `ftp://` address, which sends the password without encryption.

> [!WARNING]
> Many FTP servers let anybody in as `anonymous` or `ftp`, with any password. Organizr refuses those two names (in any letter case) before it asks the server, as if the password were wrong. Still, use a server whose accounts belong to the people who log in to Organizr.

## 📺 Emby and Jellyfin

| Setting | Key |
|---|---|
| **Emby URL** | `embyURL` |
| **Emby Token** | `embyToken` |
| **Jellyfin URL** | `jellyfinURL` |
| **Jellyfin Token** | `jellyfinToken` |

These are the same settings as for [the homepage](homepage.md). Use the local address and port of the server.

- **Emby Local** and **Jellyfin**: Organizr logs the person in at your server with `POST /Users/AuthenticateByName`, and logs that session out again right away. The login counts when the server opens a session for a user who has a password, and the logout works. A user without a password on the server can't log in to Organizr.
- **Emby Connect**: Organizr logs the person in at `connect.emby.media`, with the name or e-mail address of the Emby Connect account. Then it reads the users of your Emby server with the Emby API key (in the header `X-Emby-Token`), and looks for a user linked to that Emby Connect account. PHP Organizr notes that Emby turned this API off; if so, this way fails, and **Emby All** works like **Emby Local** alone.

## 📜 Logs

The log says why a service refused a login or couldn't be reached. It never holds a password, the bind password, an API key, or an address that contains a key. The FTP library writes each command, the password included, into its `TRACE` lines; those lines never reach the output, whatever `ORGANIZR_LOG` says.
