# 🎟️ The invites plugin

[← All documentation](README.md)

The invites plugin hands out access to your Plex server with a code. You make an invite, give the person its code, and they enter it with their Plex account. Organizr then shares your Plex server with that account. No more copying e-mail addresses into Plex by hand.

The person can also use the code to make a brand-new Plex account, or a user on your Emby server. When the plugin shares your Plex server, it can also add the person to your Plex Home and make accounts for them in Komga and Nextcloud (see [Extras after a share](#-extras-after-a-share)).

## 🔌 Switch it on

1. Log in as an Admin or a Co-Admin.
2. Open **Settings › Plugins › Inactive**, point at the card **Invites**, and select the plug button.
3. Open **Settings › Plugins › Active**, point at the card **Invites**, and select the panel button. The window **Invites Settings** opens.
4. In the section **Back end**, set **Media Server** to **Plex** (the default). Only with **Plex** does a used code share your Plex server.
5. In the section **Plex**, make sure **Plex Token** and **Plex Machine ID** both hold a value. If not, select **Retrieve** next to **Get Plex Token**, then **Retrieve** next to **Get Plex Machine**.
6. Fill in the other fields you need (see [Settings](#-settings)), and select **Save**.

Not sure which button is which? See [the plugin card buttons](bookmark-plugin.md#-switch-it-on).

The Plex token is the key of the Plex server's **owner**. The Plex Machine ID names your Plex server. **Settings › System Settings › SSO**, section **Plex**, has the same two fields (there the token is labeled **API Key/Token**): change one, and both pages show the change.

## 🔧 Settings

### 🎛️ Back end

| Label | Key | Default | What it does |
|---|---|---|---|
| Media Server | `INVITES-type-include` | Plex | **Plex** shares your Plex server when a code is used. With **Emby** or **N/A**, "I have a Plex account" only marks the code as used and shares nothing |
| Minimum Authentication | `INVITES-Auth-include` | Co-Admin | The group with the least access that can make invites |
| Allow users to delete invites | `INVITES-allow-delete-include` | Off | Lets users delete their own invites. Admins and Co-Admins can always delete. Keep it off to enforce the limit below |
| Maximum number of invites permitted for users. | `INVITES-maximum-invites` | `0` | The most invites one user can have. `0` means no limit. Admins and Co-Admins have no limit |

### 🟧 Plex

| Label | Key | What it does |
|---|---|---|
| Plex Token | `plexToken` | The server owner's Plex token. **Get Plex Token** fetches it |
| Plex Machine ID | `plexID` | Your Plex server's id. **Get Plex Machine** fetches it |
| Libraries | `INVITES-plexLibraries` | The library ids to share. Empty shares every library |
| TV Labels (comma separated), Movies Labels (comma separated), Music Labels (comma separated) | `INVITES-plex-tv-labels`, `INVITES-plex-movies-labels`, `INVITES-plex-music-labels` | With labels, the person sees only items with one of these labels. Empty means all |
| When user subscribe add him to Plex Home | `INVITES-add-plex-home` | Also invites the person to your Plex Home (see [Plex Home](#-plex-home)) |

### 🟩 Emby

These fields are used when the person picks **Make an Emby user**.

| Label | Key | What it does |
|---|---|---|
| Emby API key | `embyToken` | An API key of your Emby server |
| Emby server address | `embyURL` | The address of your Emby server |
| Emby User to be used as template for new users | `INVITES-EmbyTemplate` | The new user copies this user's policy |

The Komga and Nextcloud sections are explained under [Extras after a share](#-extras-after-a-share).

## 🚀 Use it

### ✉️ Make an invite

1. Open the user menu at the top right, and select **Invites**.
2. Enter the **Name of the person** and their **E-mail address**.
3. Select **Make an invite**. The table shows the invite with its code: six random letters and digits.
4. Give the code to the person.

When Organizr can send e-mail, the person also gets an e-mail with the code and a button. The button opens the invite page with the code filled in. [E-mail](email.md) explains how to set up e-mail and change the text of this message.

### 🔓 Use a code

The person you invited does this. They don't need an Organizr account.

1. Open Organizr. On the login page, select **Use an invite code**.
2. Enter the code. Letter case doesn't matter.
3. Pick one of the three choices:

   | Choice | What to enter | What happens |
   |---|---|---|
   | I have a Plex account | Their Plex user name or e-mail address | Your Plex server is shared with that account |
   | Make a Plex account | User name, e-mail address, password | Plex makes a new account, and your server is shared with it |
   | Make an Emby user | User name, e-mail address, password | Your Emby server gets a new user, linked to the Emby Connect account of the e-mail address |

4. Select **Use the code** (or **Make the account**).
5. For Plex: accept the invitation that Plex sends, in Plex or in the e-mail from Plex.

A code works once. When Plex refuses the share, the code stays usable, and the page shows why (see [Troubleshooting](#-troubleshooting)).

## 👤 Who can use it

| Who | Can do |
|---|---|
| Admin, Co-Admin | Change the settings. See and delete every invite. Make invites with no limit |
| Users of the **Minimum Authentication** group or above | Make invites, up to **Maximum number of invites permitted for users.**, and see their own. Delete them only with **Allow users to delete invites** |
| Anybody, logged in or not | Use a code on the **Use an invite code** page |

## 🔒 Keep invites safe

- Codes the **Invites** page makes are random. If you make one through the API with a code of your own, make it long and random: a short, made-up code is easy to guess.
- Wrong codes count like failed logins, **per address**. When the count reaches **Max Login Attempts** (`loginAttempts`), that address can't use any code for the time in **Login Lockout Seconds** (`loginLockout`). Both are in **Settings › System Settings › Login & Security**.
- Behind a reverse proxy that Organizr doesn't trust, every visitor seems to come from the proxy's address. Then one person guessing codes locks *everybody* out of invites. The same happens to a household behind one router. Set `ORGANIZR_TRUSTED_PROXIES` (see [Reverse proxy](reverse-proxy.md)).

## 🎁 Extras after a share

### 🏠 Plex Home

With **When user subscribe add him to Plex Home** switched on, a used invite also invites the invite's e-mail address to the Plex Home of the Plex token's owner. The person accepts that invitation at Plex. Plex decides who may join a Home. When it refuses, the [log](log.md) has the reason, and the invite stays used.

> [!WARNING]
> A member of your Plex Home gets more rights than a friend with a share. Switch this on only if you trust everyone you invite.

### 📖 Komga

When your Plex server is shared with the invited person, the plugin can also make them an account in [Komga](https://komga.org/). The account gets the invite's e-mail address, a first password, roles and libraries. Set it up in the section **Komga**:

| Label | Key | What it does |
|---|---|---|
| Enable Komga for auto create account | `INVITES-komga-enabled` | The switch |
| URL | `INVITES-komga-uri` | The address of Komga, for example `http://komga:25600` |
| Komga Api Key | `INVITES-komga-api-key` | An API key of a Komga Admin account. Make it in Komga, in the account's settings |
| Default password for new user | `INVITES-komga-default-user-password` | The first password of each new account. Tell the person to change it |
| Roles | `INVITES-komga-roles` | Administrator, File download, Page streaming, Kobo Sync, Koreader Sync |
| Libraries | `INVITES-komga-libraryIds` | The libraries the new account sees. The list comes from Komga when the settings open |

Every field needs a value, as in PHP Organizr; otherwise no account is made, and the log says why. The invite counts as used even when Komga makes no account.

### ☁️ Nextcloud

When your Plex server is shared with the invited person, the plugin can also make them a user in [Nextcloud](https://nextcloud.com/). The user gets the invite's e-mail address as id and address, the name you typed for the invite, and a random password nobody is told. The person sets a password with Nextcloud's "Forgot password" link, or logs in through Nextcloud's Plex app. Set it up in the section **Nextcloud**:

| Label | Key | What it does |
|---|---|---|
| Enable Nextcloud for auto create account | `INVITES-nextcloud-enabled` | The switch |
| Enable if you have Plex SSO app installed on Nextcloud | `INVITES-nextcloud-plex-sso` | The user id becomes `PlexTv-` plus the id of Plex's invite for that address, which Nextcloud's Plex app expects |
| Nextcloud URI | `INVITES-nextcloud-url` | The address of Nextcloud |
| Nextcloud Admin User, Nextcloud Admin Password | `INVITES-nextcloud-admin-user`, `INVITES-nextcloud-admin-password` | A Nextcloud Admin account that makes the users |
| Storage quota for user after subscription (empty for no-limit) | `INVITES-nextcloud-quota` | For example `10GB`. Empty for no limit |
| Nextcloud Groups after subscription | `INVITES-nextcloud-groups-member` | The groups the user joins. The list comes from Nextcloud when the settings open |

The invite counts as used even when Nextcloud makes no user, for example because that id already exists. The log has the reason.

## 🩺 Troubleshooting

| Message on the page | What it means |
|---|---|
| Code is invalid | No invite has this code, or it was used already |
| Plex User already has access | The account already has your server. The code is now used |
| Incorrect Token | Plex refused the Plex token |
| Libraries not setup correct | Plex doesn't know the Plex Machine ID, or one of the library ids |
| Plex Token/ID not set | **Plex Token** or **Plex Machine ID** is empty |
| Plex username or email not supplied | The Plex account field is empty |
| User has joined Plex, but has no access yet: … | The new Plex account exists, but the share failed. The code stays usable: use it again with **I have a Plex account** |
| Emby URL/Token not set | **Emby server address** or **Emby API key** is empty |
| User is locked out | This address sent too many wrong codes. Wait for the **Login Lockout Seconds** to pass (see [Keep invites safe](#-keep-invites-safe)) |
| Maximum number of invites reached | The user has as many invites as **Maximum number of invites permitted for users.** allows |

The code was used but nothing was shared? Check that **Media Server** is **Plex**.

## 📚 Details

### 🔗 The routes

The plugin's routes live under `/api/v2/plugins/invites`.

| Route | For | What it does |
|---|---|---|
| `GET /plugins/invites` | The plugin's group and above | The invites the caller can see |
| `POST /plugins/invites` | The plugin's group and above | Makes an invite from `code`, `username` and `email` |
| `DELETE /plugins/invites/{code}` | The plugin's group and above | Deletes an invite |
| `GET /plugins/invites/{code}` | Everybody | Says whether a code is usable |
| `POST /plugins/invites/{code}` | Everybody | Uses a code for the Plex account in `usedby` |
| `GET /plugins/invites/komga/libraries` | Admin, Co-Admin | Komga's libraries as a list of `label` and `value`, for the settings. Empty when Komga isn't set up or doesn't answer |
| `GET /plugins/invites/nextcloud/groups` | Admin, Co-Admin | Nextcloud's groups as a list of `label` and `value`, for the settings. Empty when Nextcloud isn't set up or doesn't answer |

**Make a Plex account** and **Make an Emby user** use `POST /api/v2/plex/register` and `POST /api/v2/emby/register`, with the invite code in the field `code`.

A code holds letters, digits, `-` and `_`, and has 50 characters at most.

### 🐘 Invites from PHP Organizr

An import brings your invites along with their ids, and an unused code keeps working. [Importing](importing.md) says which invites the import leaves out.
