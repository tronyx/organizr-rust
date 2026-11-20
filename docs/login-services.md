# Logging in with LDAP, FTP, Emby or Jellyfin

Organizr can check the password of a login with a server of yours: an LDAP directory (for example Active Directory, OpenLDAP or FreeIPA), an FTP server, an Emby server, Emby Connect, or a Jellyfin server. The people then log in with the name and the password that they have there. For Plex, see [Logging in with a Plex account](plex-login.md).

## Set it up

1. Log in as an Admin.
2. Open **Settings**, and select the tab **Login**.
3. Select how a password is checked:

   | Setting | Meaning |
   |---|---|
   | Organizr | Only the passwords that Organizr stores. The settings of the services do not show. |
   | The service, and then Organizr | The service first. When the service says no, or cannot be reached, the password that Organizr stores. |
   | The service only | Only the service. |

4. Select the service. The settings of that service show below.
5. Fill in the settings of the service, and select **Save**.

Do not select "The service only" before an account of the service can log in as an Admin. With that setting, Organizr checks no password that it stores. When the service cannot be reached, nobody can log in, the Admin of the set-up too. "The service, and then Organizr" keeps the Admin's own password working.

| Service | `authBackend` | What checks the password |
|---|---|---|
| LDAP | `ldap` | A bind with the name of the person at the LDAP server |
| FTP | `ftp` | A login at the FTP server |
| Emby: the Emby server | `emby_local` | A login at the Emby server of the settings |
| Jellyfin | `jellyfin` | A login at the Jellyfin server of the settings |
| Emby: Emby Connect | `emby_connect` | A login at Emby Connect, for an account that is linked to a user of the Emby server |
| Emby: the Emby server, then Emby Connect | `emby_all` | The Emby server first, and Emby Connect when the server says no |

## What a login does

1. Organizr sends the name and the password to the service. For every service but the three of Emby, the name is first written in lower case, as in PHP Organizr.
2. When the service says yes, Organizr looks for its own account with that name, or with that e-mail address, in any letter case.
3. If there is no such account, Organizr makes one in the group of new users. The name is the name that the person typed; for Emby Connect, it is the name of the Emby Connect account. The e-mail address is the `mail` of the LDAP entry, or the address of the Emby Connect account. For the other services, the account gets an address that only stands in for one, as in PHP Organizr.
4. The password of the login becomes the password of the Organizr account. So the person can log in when the service cannot be reached, if the setting allows the passwords of Organizr.
5. An Organizr account with a [second factor](second-factor.md) needs its code, as in every login.

A login that the service refuses counts as a failed login. After the number of failed logins of the setting **Failed logins before a lock**, the address of the client and the name are locked out for a time, whatever the service says.

A login with an empty password is refused without asking the LDAP or the FTP server. Many servers take an empty password as a login without a name.

## LDAP

| Setting | Key | Example |
|---|---|---|
| Address of the server | `authBackendHost` | `ldap://ldap.example.com:389` |
| Base DN | `authBaseDN` | `dc=example,dc=com` |
| Account prefix | `authBackendHostPrefix` | `uid=` for OpenLDAP, `EXAMPLE\` for Active Directory |
| Account suffix | `authBackendHostSuffix` | `,ou=people,dc=example,dc=com` for OpenLDAP, `@example.com` for Active Directory |
| Bind user name | `ldapBindUsername` | `cn=organizr,dc=example,dc=com` |
| Bind password | `ldapBindPassword` | Stored sealed, as every secret |
| Type of the LDAP server | `ldapType` | `1` Active Directory, `2` OpenLDAP, `3` FreeIPA |
| Use LDAP over TLS (ldaps) | `ldapSSL` | On or off |
| Turn the connection to TLS (StartTLS) | `ldapTLS` | On or off |

The address can name more servers, with commas between them. Organizr tries them in this order and uses the first one that answers. Every server is asked on the port of the first address. Without a port, the port is 389 for `ldap://` and for an address without a scheme, and 636 for another scheme. Whether TLS is used is the switch **Use LDAP over TLS**, not the scheme, as in PHP Organizr.

A login does three steps, each within 5 seconds:

1. Organizr binds with the bind user name and the bind password. Without a bind user name, it binds without a name.
2. Organizr binds with the prefix, the name of the person and the suffix, and the password of the login. The password is right when this bind succeeds. The line **Name of an account named TestAcct** shows the name that the server gets.
3. Organizr binds with the bind account again, and reads `mail` of the entry with the name of step 2. Without such an entry, for example with the `@example.com` form of Active Directory, the new account gets no address of LDAP.

A user name with a character that has a meaning in an LDAP name or filter (`, + " \ < > ; = * ( )`, a control character, a space at the start or the end, or `#` at the start) is refused. So a user name cannot change the entry that Organizr asks for.

With TLS, Organizr checks the certificate of the server against the certificate authorities of the system. A server with a certificate of your own authority needs that authority in the system of the computer or the container that runs Organizr. PHP Organizr accepts any certificate.

The **Type of the LDAP server** is kept for PHP Organizr. Organizr sends the same requests for the three types.

### The two tests

| Button | Route | Body | Answer |
|---|---|---|---|
| **Test the connection of LDAP** | `POST /test/ldap` | None | `200 "LDAP connection successful"`, `409` with the error of the LDAP server, or `422 "authBaseDN and/or BackendHost not supplied"` |
| **Test a login** | `POST /test/ldap/login` | `{"username": "robin", "password": "..."}` | `200 "LDAP connection successful"`, `401 "Username/Password Failed to authenticate"`, `422 "Username and/or Password not supplied"`, or `500` with the error of the LDAP server |

Both tests use the stored settings, so save the settings before you test. The second test does steps 1 and 2 of a login. The user name and the password of the test are not stored. Only an Admin or a Co-Admin can run a test.

## FTP

| Setting | Key | Example |
|---|---|---|
| Address of the server | `authBackendHost` | `ftps://ftp.example.com:21` |

The address names one server. Without a scheme, Organizr uses FTPS (`AUTH TLS` before the login), as PHP Organizr does when PHP has TLS. Without a port, the port is 21. Another scheme than `ftp` or `ftps` refuses every login.

Organizr sends `USER` and `PASS`, and `QUIT` at the end. The password is right when the server answers `230`. The whole login has 20 seconds. With FTPS, the certificate of the server is checked, as for LDAP. The owner of the project decided on 2026-11-20 to keep this check. A server with a self-signed certificate needs a certificate that the system trusts, or the plain address `ftp://` (for LDAP, `ldap://` with both TLS switches off), which sends the password without TLS.

Many FTP servers let anybody in as `anonymous` or `ftp` with any password. Organizr refuses these two names, in any letter case, before it asks the server, as a wrong password. All the same, use a server whose accounts are those of the people who log in.

## Emby and Jellyfin

| Setting | Key |
|---|---|
| Address of Emby | `embyURL` |
| API key of Emby | `embyToken` |
| Address of Jellyfin | `jellyfinURL` |
| API key of Jellyfin | `jellyfinToken` |

These are the same settings as for [the homepage](homepage.md). Use the local address and port of the server.

- **The Emby server** and **Jellyfin**: Organizr logs the person in at the server with `POST /Users/AuthenticateByName`, and logs the session out again at once. The login counts when the server opens a session of a user who has a password, and the log-out succeeds. A user without a password at the server cannot log in to Organizr.
- **Emby Connect**: Organizr logs the person in at `connect.emby.media`, with the name or the e-mail address of the Emby Connect account. Then it reads the users of the Emby server with the API key of Emby, in the header `X-Emby-Token`, and looks for a user that is linked to that Emby Connect account. PHP Organizr notes that Emby turned this API off; then this way fails, and **the Emby server, then Emby Connect** works as the Emby server alone.

## Logs

The log says why a service refused a login or could not be reached. It never holds a password, the bind password, an API key, or an address that holds a key. The library of FTP writes each command to the server in its `TRACE` lines, the password too; those lines never reach the output, whatever `ORGANIZR_LOG` says.
