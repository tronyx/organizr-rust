# 🎞️ The Plex Libraries plugin

[← All documentation](README.md)

The Plex Libraries plugin shows which libraries of your Plex server each shared account can see, with a switch per library. Admins and Co-Admins change them for every account. Your users see their own share, and can switch the libraries that you allow.

PHP Organizr has no such built-in plugin. Its marketplace had one with the same aim by another author, without a license, so this one is written from scratch against the plex.tv API and shares no code with it (D-121).

## 📋 Before you start

The plugin talks to Plex with the **Plex token** and the **Plex Machine ID** that Organizr already knows. You find both in **Settings › System Settings › SSO**, section **Plex**, where the token is labeled **API Key/Token**. When Plex is your sign-in backend, **Settings › System Settings › Sign-in**, section **Sign-in method**, shows the same two fields. The [invites plugin](invites-plugin.md) uses them too.

The token must belong to the **owner** of the Plex server. A friend's token can't change shares.

## 🔌 Switch it on

1. Log in as an Admin or a Co-Admin.
2. Open **Settings › Plugins › Inactive**, point at the card **Plex Libraries**, and select the plug button.
3. Open **Settings › Plugins › Active**, point at the card **Plex Libraries**, and select the panel button. The window **Plex Libraries Settings** opens.

Not sure which button is which? See [the plugin card buttons](bookmark-plugin.md#-switch-it-on).

## ⚙️ Settings

| Section | Label | Key | Default | What it does |
|---|---|---|---|---|
| — | (the card's plug button) | `PLEXLIBRARIES-enabled` | Off | Switches the plugin on |
| Options | Minimum Authentication | `PLEXLIBRARIES-Auth-include` | `4` (User) | The group with the least access that may use the page |
| Options | Libraries that users may switch | `PLEXLIBRARIES-librariesToInclude` | None | The libraries your users may switch on and off for themselves |
| Tab | **Make the tab Plex Libraries** (button) | — | — | Adds a tab to your menu that opens the page |

The library list in the settings comes from the server's shares, so it fills only once you've shared the server with somebody.

## 🚀 Use it

1. In the section **Options**, pick **Minimum Authentication** and the **Libraries that users may switch**.
2. Select **Save**.
3. In the section **Tab**, select **Make the tab Plex Libraries**. Your menu now has a **Plex Libraries** tab.
4. Open the tab. Each share shows its libraries, each with a switch. Flip a switch to share or unshare that library.

## 👤 Who can use it

| Who | Sees | Can change |
|---|---|---|
| Admin, Co-Admin | Every share of the server, with each account's e-mail address | Every library of every share |
| Other users of the **Minimum Authentication** group or above | Their own share: the Plex account whose user name or e-mail address matches their Organizr account (letter case doesn't matter) | Only the libraries in **Libraries that users may switch**. Their other libraries stay as they are |

With no library in **Libraries that users may switch**, users see their libraries but can't change any.

## 🩺 Troubleshooting

| Problem | Why, and what to do |
|---|---|
| The library list in the settings is empty | Plex isn't set up (token or Plex Machine ID missing), Plex doesn't answer, or the server isn't shared with anybody yet |
| A user sees no share | Their Organizr user name or e-mail address doesn't match a Plex account the server is shared with |
| A user can't flip any switch | **Libraries that users may switch** is empty, or doesn't include that library |
| Changes fail | The Plex token isn't the server owner's |

## 📚 Details

### 🔗 The routes

| Route | Who | What it does |
|---|---|---|
| `GET /api/v2/plugins/plexlibraries/libraries` | Admin, Co-Admin | The server's libraries as a list of `label` and `value`, for **Libraries that users may switch**. Empty when Plex isn't set up or doesn't answer |
| `GET /api/v2/plugins/plexlibraries/shares` | The plugin's group and above | The shares the caller sees, each library with `shared` and `changeable` |
| `PUT /api/v2/plugins/plexlibraries/shares/{share}` | The plugin's group and above | Sets a share's libraries from the list `libraries` of ids |
| `GET /api/v2/plugins/plexlibraries/setup/tab` | Admin, Co-Admin | Makes the tab of the page |

The access token of a shared account, which plex.tv sends along with the shares, never leaves the server.
