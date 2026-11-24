# The Plex Libraries plugin

The Plex Libraries plugin shows which libraries of your Plex server each shared account sees, with a switch for each library. Admins and Co-Admins change them for every account. Other people see their own share, and change the libraries that you allow.

PHP Organizr has no such built-in plugin. Its marketplace had a plugin with the same aim by another author, without a license, so this one is written new from the API of plex.tv and shares nothing with it (D-121).

## Set it up

The plugin uses the Plex token and the Plex ID of **Settings**, **System Settings**, **Main**, group **Authentication**. The token must be the one of the owner of the Plex server.

1. Log in as an Admin.
2. Open **Settings** > **Plugins** > **Inactive**. Point at the card **Plex Libraries**, and select the button with the plug.
3. Select **Active**. Point at the card **Plex Libraries**, and select the button with the panel. The window **Plex Libraries Settings** opens.
4. On the tab **Options**, choose the level of the people who may use the page in **Minimum Authentication**, and the libraries that they may switch in **Libraries that users may switch**.
5. Select **Save**.
6. On the tab **Tab**, select **Make the tab Plex Libraries**. The menu then has a tab that opens the page.

| Setting | Key | Default |
|---|---|---|
| The switch of the plugin | `PLEXLIBRARIES-enabled` | Off |
| Minimum Authentication | `PLEXLIBRARIES-Auth-include` | `4`, the level of User |
| Libraries that users may switch | `PLEXLIBRARIES-librariesToInclude` | None |

The list of libraries in the settings comes from the shares of the server, so it fills once the server is shared with somebody.

## Who sees what

| Person | Sees | Changes |
|---|---|---|
| Admin, Co-Admin | Every share of the server, with the e-mail address of each account | Every library of every share |
| Another person of the level of the plugin | The own share: the account whose user name or e-mail address is the one of the person in Organizr, in any letter case | Only the libraries of **Libraries that users may switch**. The other libraries of the share stay as they are |

Without a library in **Libraries that users may switch**, people see their libraries but change none.

## The routes

| Route | Who | What |
|---|---|---|
| `GET /api/v2/plugins/plexlibraries/settings` | Admin, Co-Admin | The form of the settings |
| `GET /api/v2/plugins/plexlibraries/shares` | The level of the plugin | The shares that the caller sees, each library with `shared` and `changeable` |
| `PUT /api/v2/plugins/plexlibraries/shares/{share}` | The level of the plugin | Sets the libraries of a share from the list `libraries` of ids |
| `GET /api/v2/plugins/plexlibraries/setup/tab` | Admin, Co-Admin | Makes the tab of the page |

The access token of a shared account, which plex.tv sends with the shares, never leaves the server.
