# Logging in with a Plex account

Organizr can let the people who use your Plex server log in with their Plex account. They need no password of their own in Organizr.

There are two ways, and you can use both:

- **The Plex password.** A person enters the name and the password of the Plex account on the login page of Organizr. Organizr asks plex.tv whether they are right.
- **The button "Log in with Plex".** A person logs in at Plex itself, in a small window. Organizr gets a token from Plex and never sees the password.

In both ways, only an account that you shared your Plex server with can log in.

## Set it up

1. Log in as an Admin.
2. Open **Settings**, then **System Settings**, then **Sign-in**. The section **Sign-in method** opens.
3. Set **Authentication Type**:

   | Choice | Meaning |
   |---|---|
   | Organizr DB | Only the passwords that Organizr stores |
   | Organizr DB + Backend | Plex first. When Plex says no, or cannot be reached, the password that Organizr stores |
   | Backend Only | Only Plex |

4. Set **Authentication Backend** to **Plex**. The Plex fields appear.
5. Next to **API Key/Token**, select **Retrieve** under **Get Plex Token**. Log in at Plex in the small window, as the owner of the server. Organizr puts the token of the owner into the field. With the token, Organizr reads the list of the accounts that the server is shared with.
6. Select **Save**. The next step uses the saved token.
7. Select **Retrieve** under **Get Plex Machine**. If the owner has one server, Organizr puts its id into **Plex Machine ID**. If the owner has more, choose the server in the list. The field shows the id as dots; select the eye button to see it.
8. Enter the Plex user name or the e-mail address of the owner in **Plex Admin Username or Email**. The owner is on no list of shared accounts, so this setting lets the owner log in.
9. To show the button, switch **Enable Plex oAuth** on.
10. Select **Save**.

Do not select "Backend Only" before a Plex account can log in as an Admin. With that setting, the Admin account of the set-up cannot log in with its Organizr password.

| Setting | Key |
|---|---|
| Authentication Type | `authType`: `internal`, `both` or `external` |
| Authentication Backend | `authBackend`: `plex` |
| API Key/Token | `plexToken` |
| Plex Machine ID | `plexID` |
| Plex Admin Username or Email | `plexAdmin` |
| Strict Plex Friends | `plexStrictFriends` |
| Enable Plex oAuth | `plexoAuth` |

## What a login does

1. Plex confirms the account: by its password, or by the token of the small window.
2. Organizr looks for the account in the list of the accounts that the server is shared with. With **Strict Plex Friends** off, every friend of the owner passes, also one who has only another server of the owner.
3. Organizr looks for its own account with the user name of the Plex account, or else with its e-mail address. An Admin or Co-Admin account is used only for the owner named in **Plex Admin Username or Email**. For any other Plex account with the name or the address of an Admin or a Co-Admin, the login fails: anyone can make a Plex account with any free name.
4. If there is no such account, Organizr makes one in the group of new users.
5. Organizr stores the Plex token of the user, sealed. [Single sign-on](single-sign-on.md) uses it.

After a login with the Plex password, that password is also the password of the Organizr account. So the person can log in when plex.tv cannot be reached, if the setting allows the passwords of Organizr. The password of an Admin or a Co-Admin account never changes this way: it stays the one that was set in Organizr.

An account that Organizr made for a login with the button has a random password that nobody knows. The person can get a password of their own with **Forgot your password?**, if the install sends [e-mail](email.md).

## The second factor

An Organizr account with a [second factor](second-factor.md) needs its code after Plex confirmed the account. The setting **Ignore 2FA if Plex OAuth** removes the code for that way of logging in: Plex has its own second factor.

## What the browser does for the button

The browser of the person talks to `plex.tv` directly: it asks for a PIN, opens the login page of Plex in a small window, and waits for the token of the PIN. Then it sends the token to Organizr. Organizr asks plex.tv whose token it is.

The page of Organizr is allowed to connect to `https://plex.tv` and to no other site.

## Other services

Organizr can also check passwords with LDAP, FTP, Emby, Emby Connect and Jellyfin. See [Logging in with LDAP, FTP, Emby or Jellyfin](login-services.md).
