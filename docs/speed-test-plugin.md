# The speed test plugin

The speed test plugin measures the connection between a browser and Organizr: the download speed, the upload speed, and the time of a request.

The test tells you about the path from your device to the server of Organizr. It does not measure the internet connection of the server.

## Set it up

1. Log in as an Admin.
2. Open **Settings**, select the tab **Plugins**, and select **Inactive**.
3. Point at the card **SpeedTest**, and select the button with the plug.
4. Select **Active**. Point at the card **SpeedTest**, and select the button with the panel. The window **SpeedTest Settings** opens.
5. In **Minimum Authentication**, select the level of the users who can run the test.
6. Select **Save**.

| Setting | Key |
|---|---|
| Use the plugin | `SPEEDTEST-enabled` |
| Level | `SPEEDTEST-Auth-include` |

## Run a test

1. Open the menu of the user, at the top right.
2. Select **Speed test**.
3. Select **Start**.

A run takes about twenty seconds. It shows four values:

| Value | Meaning |
|---|---|
| Download | Megabits per second from the server to the browser |
| Upload | Megabits per second from the browser to the server |
| Ping | The shortest time of a request, in milliseconds |
| Jitter | How much the time of a request changes, in milliseconds |

The page also shows the address that the server sees your browser at. Behind a reverse proxy, this is your own address only when Organizr trusts the proxy.

## Good to know

- A run moves much data: on a fast line, some gigabytes. Do not run it on a connection that you pay for by volume.
- A reverse proxy that limits the size of a request can make the upload part fail. A run sends requests of up to 32 MiB. For nginx, the setting is `client_max_body_size`.
- A reverse proxy that keeps a whole answer before it sends it on makes the download look faster or slower than it is. For nginx, switch `proxy_buffering` off for Organizr.
- On a connection of more than about one gigabit per second, the values show the speed of the browser and the server more than the speed of the line.
