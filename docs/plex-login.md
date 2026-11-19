# Logging in with a Plex account

Organizr can let the people who use your Plex server log in with their Plex account. They need no password of their own in Organizr.

There are two ways, and you can use both:

- **The Plex password.** A person enters the name and the password of the Plex account on the login page of Organizr. Organizr asks plex.tv whether they are right.
- **The button "Log in with Plex".** A person logs in at Plex itself, in a small window. Organizr gets a token from Plex and never sees the password.

In both ways, only an account that you shared your Plex server with can log in.

## Set it up

1. Log in as an Admin.
2. Open **Settings**, and select the tab **Login**.
3. Enter the Plex token of the owner of the server, and select **Save**. With the token, Organizr reads the list of the accounts that the server is shared with.
4. Select **List the Plex servers**, and copy the id of your server into the field **Id of the Plex server**.
5. Enter the Plex user name or the e-mail address of the owner. The owner is on no list of shared accounts, so this setting lets the owner log in.
6. Select how a password is checked:

   | Setting | Meaning |
   |---|---|
   | Organizr | Only the passwords that Organizr stores |
   | The service, and then Organizr | Plex first. When Plex says no, or cannot be reached, the password that Organizr stores |
   | The service only | Only Plex |

7. Select **Plex** as the service.
8. To show the button, switch **Show the button "Log in with Plex"** on.
9. Select **Save**.

Do not select "The service only" before a Plex account can log in as an Admin. With that setting, the Admin account of the set-up cannot log in with its Organizr password.

| Setting | Key |
|---|---|
| A password is checked by | `authType`: `internal`, `both` or `external` |
| Service | `authBackend`: `plex` |
| Plex token of the owner | `plexToken` |
| Id of the Plex server | `plexID` |
| Plex account of the owner | `plexAdmin` |
| Only accounts that this server is shared with | `plexStrictFriends` |
| Show the button | `plexoAuth` |

## What a login does

1. Plex confirms the account: by its password, or by the token of the small window.
2. Organizr looks for the account in the list of the accounts that the server is shared with. With **Only accounts that this server is shared with** off, every friend of the owner passes, also one who has only another server of the owner.
3. Organizr looks for its own account with the user name of the Plex account, or else with its e-mail address.
4. If there is no such account, Organizr makes one in the group of new users.
5. Organizr stores the Plex token of the user, sealed. [Single sign-on](single-sign-on.md) uses it.

After a login with the Plex password, that password is also the password of the Organizr account. So the person can log in when plex.tv cannot be reached, if the setting allows the passwords of Organizr.

An account that Organizr made for a login with the button has a random password that nobody knows. The person can get a password of their own with **Forgot your password?**, if the install sends [e-mail](email.md).

## The second factor

An Organizr account with a [second factor](second-factor.md) needs its code after Plex confirmed the account. The setting **No code for a login with the button "Log in with Plex"** removes the code for that way of logging in: Plex has its own second factor.

## What the browser does for the button

The browser of the person talks to `plex.tv` directly: it asks for a PIN, opens the login page of Plex in a small window, and waits for the token of the PIN. Then it sends the token to Organizr. Organizr asks plex.tv whose token it is.

The page of Organizr is allowed to connect to `https://plex.tv` and to no other site.

## Other services

PHP Organizr can also check passwords with Emby, Jellyfin, LDAP and FTP. This application does not have these yet.
