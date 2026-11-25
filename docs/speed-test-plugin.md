# 🏎️ The speed test plugin

[← All documentation](README.md)

The speed test plugin measures the connection between your browser and Organizr: download speed, upload speed, ping and jitter. Handy when a stream stutters and you want to know whether it's your Wi-Fi or something else.

It measures the path from your device to the Organizr server. It does **not** measure the server's own internet connection.

## 🔌 Switch it on

1. Log in as an Admin or a Co-Admin.
2. Open **Settings › Plugins › Inactive**, point at the card **SpeedTest**, and select the plug button.
3. Open **Settings › Plugins › Active**, point at the card **SpeedTest**, and select the panel button. The window **SpeedTest Settings** opens.
4. In **Minimum Authentication**, pick the group with the least access that may run the test.
5. Select **Save**.

Not sure which button is which? See [the plugin card buttons](bookmark-plugin.md#-switch-it-on).

## ⚙️ Settings

| Label | Key | Default | What it does |
|---|---|---|---|
| (the card's plug button) | `SPEEDTEST-enabled` | Off | Switches the plugin on |
| Minimum Authentication | `SPEEDTEST-Auth-include` | Co-Admin | The group with the least access that may run the test |

## 🚀 Use it

1. Open the user menu at the top right (your user name).
2. Select **Speed test**.
3. Select **Start**. A run takes about twenty seconds.

| Value | What it means |
|---|---|
| Download | Megabits per second from the server to your browser |
| Upload | Megabits per second from your browser to the server |
| Ping | The shortest time of a request, in milliseconds |
| Jitter | How much the time of a request varies, in milliseconds |

The page also shows the address the server sees your browser at. Behind a reverse proxy, that's your real address only when Organizr trusts the proxy (see [Reverse proxy](reverse-proxy.md)).

> [!CAUTION]
> A run moves a lot of data: about a hundred megabytes on a slow line, and a gigabyte or more on a fast one. Don't run it on a connection you pay for by volume.

## 👤 Who can use it

Users of the **Minimum Authentication** group and above see **Speed test** in their user menu and can run it. Everybody else gets no menu entry, and the server refuses their requests.

## 🩺 Troubleshooting

| Problem | Why, and what to do |
|---|---|
| No **Speed test** in the user menu | The plugin is off, or your group is below **Minimum Authentication** |
| The upload fails | Your reverse proxy limits the size of a request. A run sends requests of up to 32 MiB. In nginx, raise `client_max_body_size` for Organizr |
| The download looks too fast or too slow | Your reverse proxy holds the whole answer before it passes it on. In nginx, set `proxy_buffering off` for Organizr |
| The address shown is your proxy's | Organizr doesn't trust the proxy yet. See [Reverse proxy](reverse-proxy.md) |
| The values look capped around one gigabit | Above about one gigabit per second, the test shows how fast the browser and the server are more than how fast the line is |
