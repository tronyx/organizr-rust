# The invites plugin

The invites plugin gives a person access to your Plex server with a code. A member of your install makes an invite, which is a record with a code. The invited person enters the code and the name of the own Plex account. Organizr then shares the Plex server with that account.

The plugin works with Plex. The invited person can also make a new Plex account, or a user on your Emby server with the settings of the tab **Emby Settings**. The parts of PHP Organizr for Komga, Nextcloud and the Plex Home are not built yet: their tabs and switches say so, and nothing uses their settings.

## Set it up

The plugin shares the server with the Plex token and the Plex ID of the install. The Plex token is the key of the owner of the Plex server. The Plex ID is the machine identifier of the Plex server.

1. Log in as an Admin.
2. Open **Settings**, select the tab **Plugins**, and select **Inactive**.
3. Point at the card **Invites**, and select the button with the plug.
4. Select **Active**. Point at the card **Invites**, and select the button with the panel. The window **Invites Settings** opens.
5. On the tab **Plex Settings**, make sure that the fields **Plex Token** and **Plex Machine** hold a value. **System Settings**, **SSO**, tab **Plex** has the same two fields.
6. Fill in the other fields of the plugin, and select **Save**.

| Field | Meaning |
|---|---|
| Minimum Authentication | The group with the least access that can make invites |
| Maximum number of invites permitted for users. | The most invites that one user can have. The number 0 means no limit. An Admin and a Co-Admin have no limit |
| Allow users to delete invites | Without this switch, only an Admin or a Co-Admin deletes invites |
| Libraries | The ids of the libraries, with commas between them. An empty field shares every library |
| TV Labels (comma separated), Movies Labels (comma separated), Music Labels (comma separated) | With labels, the person sees only the items that have one of the labels |

## Make an invite

1. Open the menu of the user, and select **Invites**.
2. Enter the name and the e-mail address of the person.
3. Select **Make an invite**. The table shows the invite with its code.
4. Give the code to the person.

When the install sends e-mails, the person gets an e-mail with the code and a button. The button opens the page for an invite code with the code filled in. [E-mail](email.md) tells you how to set the e-mails up and how to change the text of this e-mail.

An Admin and a Co-Admin see the invites of every member. Each other member sees the own invites.

## Use a code

1. Open Organizr. A login is not necessary.
2. On the login page, select **Use an invite code**.
3. Enter the code. The letter case of the code does not matter.
4. Enter the user name or the e-mail address of the Plex account, and select **Use the code**.
5. Accept the invitation that Plex sends to the account.

A code works one time. When Plex refuses the share, the code stays good, and the page shows the reason:

| Text on the page | Meaning |
|---|---|
| Code is invalid | No invite has the code, or the code is used |
| Plex User already has access | The account has the server already. The code is used |
| Incorrect Token | Plex refused the Plex token of the install |
| Libraries not setup correct | Plex does not know the Plex ID or one of the library ids |
| Plex Token/ID not set | The install has no Plex token or no Plex ID |
| User is locked out | This browser sent too many wrong codes. See below |

## The limit for wrong codes

Wrong codes are counted as failed logins are counted. When the count reaches the number of the setting `loginAttempts`, the server refuses each code for the time of the setting `loginLockout`. This stops a person who tries codes one after the other.

## The routes

The plugin has these routes under `/api/v2/plugins/invites`.

| Route | For | Purpose |
|---|---|---|
| `GET /plugins/invites` | Members with the level of the plugin | The invites that the caller can see |
| `POST /plugins/invites` | Members with the level of the plugin | Make an invite from `code`, `username` and `email` |
| `DELETE /plugins/invites/{code}` | Members with the level of the plugin | Delete an invite |
| `GET /plugins/invites/{code}` | Everybody | Tell whether a code is good |
| `POST /plugins/invites/{code}` | Everybody | Use a code for the Plex account in `usedby` |

A code holds letters, digits, `-` and `_`, and has 50 characters at most.

## Invites of PHP Organizr

An import brings the invites with their ids, and a code that was not used stays good. [Importing](importing.md) says which invites the import leaves out.
