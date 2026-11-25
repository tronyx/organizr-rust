# 💬 The chat plugin

[← All documentation](README.md)

The chat plugin gives the people on your Organizr one shared chat room. New messages show up live, without a page reload. Great for "is Plex down for you too?" moments.

## 🔌 Switch it on

1. Log in as an Admin or a Co-Admin.
2. Open **Settings › Plugins › Inactive**, point at the card **Chat**, and select the plug button.
3. Open **Settings › Plugins › Active**, point at the card **Chat**, and select the panel button. The window **Chat Settings** opens.
4. Set **Minimum Authentication** and **# of Previous Messages** (see below), and select **Save**.

Not sure which button is which? See [the plugin card buttons](bookmark-plugin.md#-switch-it-on).

## ⚙️ Settings

| Section | Label | Key | Default | What it does |
|---|---|---|---|---|
| — | (the card's plug button) | `CHAT-enabled` | Off | Switches the plugin on |
| Options | Minimum Authentication | `CHAT-Auth-include` | Co-Admin | The group with the least access that may read and send messages |
| Options | # of Previous Messages | `CHAT-messageLoadLimit` | `200` | How many of the newest messages the page loads. Never more than 1000, whatever you enter |

> [!TIP]
> The default lets only Admins and Co-Admins chat. To open the room to everybody with an account, set **Minimum Authentication** to **User**.

## 🚀 Use it

1. Open the user menu at the top right, and select **Chat**.
2. Type your message in the box at the bottom.
3. Press **Enter** or select **Send**. **Shift+Enter** starts a new line.

Your own messages sit on the right, everybody else's on the left. On a wide screen, the column **Wrote lately** lists who wrote recently, and when.

| Limit | Value |
|---|---|
| Length of one message | 1000 characters |
| Messages per person | 10 in 30 seconds. More gets "Too many chat messages. Wait a moment, then send again." |
| Messages the page loads | **# of Previous Messages**, at most 1000 |

Messages are plain text. Links and HTML show as typed, never as a link or markup.

## 👤 Who can use it

| Who | Can do |
|---|---|
| Admin, Co-Admin | Change the settings |
| Logged-in users of the **Minimum Authentication** group or above | See **Chat** in the user menu, read and send messages |
| Guests | Never chat, even when **Minimum Authentication** is set to **Guest** |
| A program with the API key | Send a message through the API. It shows as **Organizr API** |

> [!NOTE]
> There is no button to delete a message, and Organizr keeps every message in its database. Organizr also stores the sender's address with each message, but never shows it to anyone.

## 🩺 Troubleshooting

| Problem | Why, and what to do |
|---|---|
| No **Chat** in the user menu | The plugin is off, or your group is below **Minimum Authentication** |
| New messages show only after a reload | Your reverse proxy holds the live stream back. In nginx, add the `location` block for `/api/v2/plugins/chat/events` with `proxy_buffering off` (see [Reverse proxy](reverse-proxy.md)) |
| "Too many chat messages. Wait a moment, then send again." | You sent 10 messages in 30 seconds. Wait a moment |
| "String is over limit of: 1000" (API only) | The message is longer than 1000 characters. The page's box stops at 1000 |
| "CHAT-enabled module is not enabled" | The plugin is switched off |

## 📚 Details

### 🔗 The routes

| Route | What it does |
|---|---|
| `GET /api/v2/plugins/chat/message` | The newest messages, newest first, with `username`, `gravatar`, `uid`, `date` (UTC) and `message` |
| `POST /api/v2/plugins/chat/message` | Sends the text in the field `message` |
| `GET /api/v2/plugins/chat/events` | The live stream: each new message as a server-sent event `message`. A stream ends after five minutes; the browser opens a new one, which checks the login again |

All three need the **Minimum Authentication** group (never a guest), and answer `409` while the plugin is off.

### 🐘 Chat from PHP Organizr

- An import copies your old messages, with their ids.
- PHP Organizr sends new messages through Pusher, an outside service. Here Organizr sends them itself (D-44). The Pusher settings are imported and ignored, and so are the refresh time and the sounds: the page plays no sound.
- The full list of differences is D-24, D-44 and D-95 in [Deviations](deviations.md).
