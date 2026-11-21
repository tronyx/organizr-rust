# The homepage

The homepage is a page of Organizr that shows items. An item shows data of another application, or HTML that you write.

The set-up makes a tab with the name "Homepage" and leaves it out of the menu. To use the homepage, show the tab in the menu:

1. Log in as an Admin.
2. Open **Settings**, and select the tab **Tabs**.
3. Switch **Show Homepage in the menu** on.

## The items

Each item has three settings of its own:

| Setting | Meaning |
|---|---|
| A switch | The homepage shows the item only when its switch is on. |
| A level | Only users of that level, or with more access, see the item. |
| A place | A number. The item with the lowest number stands at the top. |

A person sees an item only when all of these are true: the switch is on, the person has the level, and the item has the settings that it needs.

This version has these items:

- [Custom HTML](#custom-html)
- [Health checks](#health-checks)
- [Uptime Kuma](#uptime-kuma)
- [Calendar](#calendar), with Sonarr, Radarr, Lidarr, SickRage, CouchPotato, Trakt and calendars in the iCal format
- [Downloads of Sonarr and Radarr](#downloads-of-sonarr-and-radarr)
- [SABnzbd](#sabnzbd)
- [qBittorrent](#qbittorrent)
- [NZBGet](#nzbget)
- [Transmission](#transmission)
- [Deluge](#deluge)
- [rTorrent](#rtorrent)
- [uTorrent](#utorrent)
- [JDownloader](#jdownloader)
- [Pi-hole](#pi-hole)
- [Weather and air](#weather-and-air)
- [Plex](#plex): what plays now, new items and playlists
- [Emby](#emby): what plays now and new items
- [Jellyfin](#jellyfin): what plays now and new items
- [Tautulli](#tautulli)
- [Overseerr](#overseerr)
- [Ombi](#ombi)
- [Jackett](#jackett) and [Prowlarr](#prowlarr): a search of the indexers
- [AdGuard Home](#adguard-home)
- [Netdata](#netdata)
- [Monitorr](#monitorr)
- [Speedtest Tracker](#speedtest-tracker)
- [OctoPrint](#octoprint)
- [UniFi](#unifi)
- [PromPage](#prompage)
- [Emby live TV](#emby-live-tv)
- [JellyStat](#jellystat)
- [Donate](#donate)

You set all items up in **Settings**, on the tab **Homepage**.

The homepage loads the data of an item again after the time that the item has in its settings. The shortest time is five seconds.


## Test the connection of a service

The settings of these services have a button **Test the connection**: Plex, Emby, Jellyfin, Tautulli, Overseerr, Ombi, Sonarr, Radarr, Lidarr, SickRage, SABnzbd, qBittorrent, NZBGet, Transmission, Deluge, rTorrent, uTorrent, JDownloader, Pi-hole, Jackett, Prowlarr, AdGuard Home, UniFi, Emby live TV and JellyStat. The settings of UniFi have a second button, **Read the sites of the controller**, which is the test `unifi/site`.

1. Enter the address and the key or the login of the service.
2. Select **Save**. The test reads the settings that are stored.
3. Select **Test the connection**. The page shows whether the service answered, or what is wrong.

Only an Admin or a Co-Admin can run a test. A script can run it with `POST /api/v2/test/<service>`, where the service is the name in lower case without the hyphen, for example `pihole`. The test of Emby live TV has the name `embyLiveTVTracker`.

[Tests of the settings](settings-tests.md) describes the tests that test no service of the homepage.

## Custom HTML

There are eight custom HTML items, with the numbers 01 to 08. Each one holds HTML that you write.

1. Open **Settings**, and select the tab **Homepage**.
2. Switch **Show custom HTML 01** on.
3. Select the level of the users who see the item.
4. Enter the HTML.
5. Select **Save**.

| Setting | Key |
|---|---|
| Switch | `homepageCustomHTML01Enabled` |
| Level | `homepageCustomHTML01Auth` |
| Place | `homepageOrdercustomhtml01` |
| HTML | `customHTML01` |
| Trust the item | `homepageCustomHTML01Trusted` |

### The frame of an item

The homepage shows each custom HTML item in a frame of its own. The browser treats the content of the frame as a page of no site. Scripts of the item run. They cannot read the cookies of Organizr, and they cannot use Organizr as the person who looks at the page.

PHP Organizr writes the HTML into its own page. There, a script of an item can do all that the person can do, for example change settings when an Admin looks at the homepage.

What this means for the HTML of an item:

- The item can use styles and scripts of its own, and it can load files from other sites.
- The item can use the classes and the icons of the style sheet of this application.
- A link without a `target` changes the address of Organizr. The link `<a href="#Plex">` opens the tab "Plex".
- A link with `target="_blank"` opens a new window.
- The frame is as high as its content.

### Trusted items

An item in the frame is a page of no site. Some pages only let your own site frame them: a reverse proxy sends the header `Content-Security-Policy: frame-ancestors` with the names of your sites. Such a page stays empty in the frame, and a page that needs the login of your site does not get it.

For such an item, switch **Trust custom HTML 01** on. The item is then a page of your site, as in PHP Organizr:

- It can embed pages that only let your site frame them, and the browser sends your login to them.
- A script in the item can do all that the person who looks at the page can do in Organizr. This also holds for a script that the HTML loads from another site.

Trust only HTML and scripts that you know. The switch is off for each item at the start, also after an import.

### HTML from PHP Organizr

An import brings the HTML of your items as it is. Look at each item after the import:

- Each item starts with a small style: the dark page, the grid of Bootstrap 3 (`row`, `col-sm-4` and the like), and the classes `btn`, `panel`, `white-box`, `pull-left`, `pull-right`, `text-center` and `clearfix`. The other CSS classes of the PHP interface do not exist here. An item that uses them looks different.
- The JavaScript functions of the PHP interface do not exist in the frame. A script that calls them fails.
- A script cannot call the API of Organizr with the session of the person.

## Health checks

This item shows the checks of one or more projects of [Healthchecks](https://healthchecks.io/), the checks that are down first.

| Setting | Key |
|---|---|
| Switch | `homepageHealthChecksEnabled` |
| Level | `homepageHealthChecksAuth` |
| Place | `homepageOrderhealthchecks` |
| Address of the list of checks | `healthChecksURL` |
| API keys of the projects | `healthChecksToken` |
| Only checks with these tags | `healthChecksTags` |
| Show the notes of a check | `homepageHealthChecksShowDesc` |
| Show the tags of a check | `homepageHealthChecksShowTags` |
| Do not check the certificate | `healthChecksDisableCertCheck` |
| Milliseconds between two loads | `homepageHealthChecksRefresh` |

The address is the address of the list of checks, for example `https://healthchecks.io/api/v1/checks`. For your own install of Healthchecks, replace the host.

Each project has an API key of its own. For more than one project, enter the keys with commas between them. A read-only key is enough.

A project that does not answer is left out. The item then shows the checks of the other projects.

## Uptime Kuma

This item shows the monitors of an install of [Uptime Kuma](https://uptime.kuma.pet/), and whether each one is up.

| Setting | Key |
|---|---|
| Switch | `homepageUptimeKumaEnabled` |
| Level | `homepageUptimeKumaAuth` |
| Place | `homepageOrderUptimeKuma` |
| Address of Uptime Kuma | `uptimeKumaURL` |
| API key | `uptimeKumaToken` |
| Show a title | `homepageUptimeKumaHeaderToggle` |
| Title | `homepageUptimeKumaHeader` |
| Small form | `homepageUptimeKumaCompact` |
| Show the response time | `homepageUptimeKumaShowLatency` |
| Milliseconds between two loads | `homepageUptimeKumaRefresh` |

Make the API key in the settings of Uptime Kuma, under **API Keys**. Organizr reads the page `/metrics` of Uptime Kuma with the key.

A monitor that Uptime Kuma did not check yet is not shown. A monitor in maintenance is shown as online.

## Calendar

The calendar shows the episodes of Sonarr and SickRage, the films of Radarr and CouchPotato, the albums of Lidarr, the episodes and the films of a Trakt account, and the events of calendars in the iCal format, for the days around today.

The calendar has no switch of its own. It shows when a person can see one of its sources at least. Sonarr, Radarr and Lidarr have these settings, here with the keys of Sonarr:

| Setting | Key |
|---|---|
| Switch | `homepageSonarrEnabled` |
| Level | `homepageSonarrAuth` |
| Address | `sonarrURL` |
| API key | `sonarrToken` |
| Do not check the certificate | `sonarrDisableCertCheck` |
| Address that the link of an event opens | `sonarrCalendarLink` |
| Show the link in an event | `sonarrIcon` |
| Show what Sonarr does not monitor | `sonarrUnmonitored` |

For more than one install of an application, enter the addresses with commas between them. Enter one API key for each address, in the same order.

Radarr has three more switches: `radarrPhysicalRelease`, `radarrDigitalRelease` and `radarrCinemaRelease`. A film has one event for each release that is switched on. Releases on the same day are one event.

The calendar itself has these settings:

| Setting | Key |
|---|---|
| Place | `homepageOrdercalendar` |
| View at the start: `month`, `basicWeek`, `basicDay` or `list` | `calendarDefault` |
| First day of a week | `calendarFirstDay` |
| Days before today that are loaded | `calendarStart` |
| Days after today that are loaded | `calendarEnd` |
| Most events that a day of the month shows | `calendarLimit` |
| Milliseconds between two loads | `calendarRefresh` |

The calendar has four views: the month, the week, the day and the list. The buttons with the arrows go one month, one week or one day back or on. The button "Filter" shows the switches for the kinds and the states of events.

An event has a colour for its state:

| State | Meaning |
|---|---|
| Downloaded | The file is there. |
| Not released yet | The release is in the future. |
| Premiere | The first episode of a season, which was not aired yet. |
| Not monitored | An episode that was not aired yet, and that Sonarr does not monitor. |
| Missing | The release is in the past, and there is no file. |
| Upcoming | An event of a calendar in the iCal format that is not over. |
| Over | An event of a calendar in the iCal format that is over. |

The day of a film is a day of the time zone of the server. Set the environment variable `TZ` of the container to your time zone, for example `TZ=Europe/Berlin`.

### SickRage

The calendar shows three groups of episodes from SickRage: the episodes that were missed, the episodes of today and of the days to come, and the last 100 episodes that were downloaded. Applications that have the API of SickRage work too, for example SickChill.

| Setting | Key |
|---|---|
| Switch | `homepageSickrageEnabled` |
| Level | `homepageSickrageAuth` |
| Address | `sickrageURL` |
| API key | `sickrageToken` |
| Do not check the certificate | `sickrageDisableCertCheck` |

For more than one install, enter the addresses with commas between them. Enter one API key for each address, in the same order.

SickRage gives the time of an episode without a time zone. The calendar takes it for a time of the time zone of the server.

An episode of SickRage shows without a picture.

### CouchPotato

The calendar shows the films of CouchPotato whose release is in the days that the calendar loads. A film that CouchPotato has shows as "Downloaded".

| Setting | Key |
|---|---|
| Switch | `homepageCouchpotatoEnabled` |
| Level | `homepageCouchpotatoAuth` |
| Address | `couchpotatoURL` |
| API key | `couchpotatoToken` |
| Do not check the certificate | `couchpotatoDisableCertCheck` |

For more than one install, enter the addresses with commas between them. Enter one API key for each address, in the same order.

CouchPotato has no connection test.

### Calendars in the iCal format

The calendar shows the events of calendar files in the iCal format (`.ics`). Most calendar services give such an address for a calendar, for example Google Calendar and Nextcloud.

| Setting | Key |
|---|---|
| Switch | `homepageCalendarEnabled` |
| Level | `homepageCalendarAuth` |
| Addresses of the calendars | `calendariCal` |

For more than one calendar, enter the addresses with commas between them. An address that starts with `webcal://` is asked over HTTPS.

The address of a private calendar holds a key. Organizr does not write the address into its log or into an error text.

The calendar shows an event when its start is in the days that the calendar loads. An event shows as "Upcoming" until its end, and as "Over" after that.

An event that repeats shows on each day of its rule. Organizr knows these rules:

| Rule | Example |
|---|---|
| Each day, or every few days | `FREQ=DAILY;INTERVAL=2` |
| Each week, on the day of the start or on named days | `FREQ=WEEKLY;BYDAY=MO,TH` |
| Each month, on the day of the start or on named days | `FREQ=MONTHLY;BYMONTHDAY=1,15` |
| Each month, on a day of a week | `FREQ=MONTHLY;BYDAY=2TU` or `BYDAY=-1FR` |
| Each year | `FREQ=YEARLY` |
| A number of times, or until a date | `COUNT=10` or `UNTIL=20270101T000000Z` |

An event with another rule shows once, on the day of its first start. Dates that a calendar takes out of a rule (`EXDATE`) still show.

A time of an event is read in this order:

1. A time in UTC is that moment.
2. A time with a time zone (`TZID`) is a time of that time zone.
3. In a calendar that names a time zone for itself (`X-WR-TIMEZONE`), a time is a time of that time zone.
4. Other times are times of the time zone of the server.

A script gets the events of one source with `GET /api/v2/homepage/sickrage/calendar`, `GET /api/v2/homepage/couchpotato/calendar` or `GET /api/v2/homepage/ical/calendar`. `GET /api/v2/homepage/calendar` has the events of the applications in `events`, and the events of the iCal addresses in `ical`.

### Trakt

The calendar shows the episodes and the films of the calendar of one [Trakt](https://trakt.tv/) account.

| Setting | Key |
|---|---|
| Switch | `homepageTraktEnabled` |
| Level | `homepageTraktAuth` |
| Client id of the API application | `traktClientId` |
| Client secret of the API application | `traktClientSecret` |
| Days before today that are loaded | `calendarStartTrakt` |
| Days after today that are loaded | `calendarEndTrakt` |

Organizr needs an API application at Trakt, and the permission of the account:

1. Log in at Trakt, and open the page **Your API Apps** of the settings. Add a new application.
2. Enter a name. For the redirect URI, enter the address of Organizr with `api/v2/oauth/trakt` after it, for example `https://organizr.example.com/api/v2/oauth/trakt`.
3. Copy the client id and the client secret of the application.
4. In Organizr, open **Settings**, and select the tab **Homepage**. Enter the client id and the client secret in the group **Trakt**, and select **Save**.
5. Select **Connect the Trakt account**. A new window opens the page of Trakt.
6. Allow the access at Trakt. The window comes back to Organizr and shows **Details Saved**.
7. Switch **Show Trakt in the calendar** on, and select **Save**.

Only an Admin or a Co-Admin can connect the account. Organizr stores the access token and the refresh token of the account as secrets. The settings page does not show them.

- When the access token is over, Organizr asks Trakt for new tokens before it loads the calendar. You do not connect the account again.
- If Trakt refuses the tokens, the calendar shows no event of Trakt. Select **Connect the Trakt account** again.
- Trakt gives 33 days at most: the days before today and the days after today together.
- An event of Trakt has no picture.
- Trakt does not know your files. An episode that was aired shows as missing. An episode and a film to come show as not monitored. The first episode of a season to come shows as a premiere.
- The address of Organizr in the redirect URI must be the one that your browser uses, with `https` when your browser uses it.

A script gets the events with `GET /api/v2/homepage/trakt/calendar`.

### Pictures of events

An event shows the fan art of its series or film. When only Sonarr or Radarr has the picture, a browser cannot load it without the API key. The server then fetches the picture, keeps it in the folder `cache` of the data folder, and sends it from `data/cache/`. The first time, the event shows without its picture for a moment.

You can delete the folder `cache` at any time. The server fetches the pictures again.

## Downloaders in one box

Each downloader has a switch "combine". The downloaders whose switch is on share one item, as in PHP Organizr. The item has a tab for each of them, with its picture and the number of its entries. The tabs stand in the order of the places of the downloaders.

| Setting | Key |
|---|---|
| Place of the shared item | `homepageOrderdownloader` |
| Switch for SABnzbd | `sabnzbdCombine` |
| Switch for NZBGet | `nzbgetCombine` |
| Switch for Transmission | `transmissionCombine` |
| Switch for qBittorrent | `qBittorrentCombine` |
| Switch for Deluge | `delugeCombine` |
| Switch for rTorrent | `rTorrentCombine` |
| Switch for uTorrent | `uTorrentCombine` |
| Switch for JDownloader | `jdownloaderCombine` |
| Switch for the downloads of Sonarr | `homepageSonarrQueueCombine` |
| Switch for the downloads of Radarr | `homepageRadarrQueueCombine` |

A list of downloads is at most 300 pixels high. A longer list scrolls.

## Downloads of Sonarr and Radarr

These two items show the downloads in the queue of Sonarr and of Radarr. Each one needs the switch of its application and a switch of its own.

| Setting | Key for Sonarr |
|---|---|
| Switch | `homepageSonarrQueueEnabled` |
| Level | `homepageSonarrQueueAuth` |
| Place | `homepageOrderSonarrQueue` |
| Milliseconds between two loads | `homepageSonarrQueueRefresh` |

A person needs the level of the application and the level of its downloads.

## SABnzbd

This item shows the downloads in the queue of SABnzbd, and the last downloads that are done.

| Setting | Key |
|---|---|
| Switch | `homepageSabnzbdEnabled` |
| Level | `homepageSabnzbdAuth` |
| Place | `homepageOrdersabnzbd` |
| Address | `sabnzbdURL` |
| API key | `sabnzbdToken` |
| Do not check the certificate | `sabnzbdDisableCertCheck` |
| Milliseconds between two loads | `sabnzbdRefresh` |

An Admin or a Co-Admin can pause and resume the whole queue, or one download. Other users see the queue only.

Use the full API key of SABnzbd. The NZB key cannot read the queue.

## qBittorrent

This item shows the torrents of qBittorrent and how far each one is.

| Setting | Key |
|---|---|
| Switch | `homepageqBittorrentEnabled` |
| Level | `homepageqBittorrentAuth` |
| Place | `homepageOrderqBittorrent` |
| Address | `qBittorrentURL` |
| User name | `qBittorrentUsername` |
| Password | `qBittorrentPassword` |
| Version of the API | `qBittorrentApiVersion` |
| Sorted by | `qBittorrentSortOrder` |
| Turn the order around | `qBittorrentReverseSorting` |
| Leave out the torrents that only upload | `qBittorrentHideSeeding` |
| Leave out the torrents that are complete and paused | `qBittorrentHideCompleted` |
| Do not check the certificate | `qBittorrentDisableCertCheck` |
| Milliseconds between two loads | `qBittorrentRefresh` |

Select version 2 of the API for qBittorrent 4.1 and later. The default of the setting is version 1, as in PHP Organizr.

Organizr logs in to qBittorrent with the user name and the password each time it loads the torrents. If qBittorrent blocks an address after wrong logins, a wrong password in this setting blocks the address of Organizr.


## NZBGet

This item shows the downloads in the queue of NZBGet, and its last ten downloads.

| Setting | Key |
|---|---|
| Switch | `homepageNzbgetEnabled` |
| Level | `homepageNzbgetAuth` |
| Place | `homepageOrdernzbget` |
| Address | `nzbgetURL` |
| User name | `nzbgetUsername` |
| Password | `nzbgetPassword` |
| Do not check the certificate | `nzbgetDisableCertCheck` |
| Milliseconds between two loads | `nzbgetRefresh` |

Organizr sends the user name and the password only when both hold a value.

## Transmission

This item shows the torrents of Transmission and how far each one is. The oldest torrent is first.

| Setting | Key |
|---|---|
| Switch | `homepageTransmissionEnabled` |
| Level | `homepageTransmissionAuth` |
| Place | `homepageOrdertransmission` |
| Address | `transmissionURL` |
| User name | `transmissionUsername` |
| Password | `transmissionPassword` |
| Leave out the torrents that upload | `transmissionHideSeeding` |
| Leave out the torrents that are stopped | `transmissionHideCompleted` |
| Do not check the certificate | `transmissionDisableCertCheck` |
| Milliseconds between two loads | `transmissionRefresh` |

The address is the address of the web interface of Transmission without `/rpc` at its end. For a default install, this is `http://<host>:9091/transmission`.

## Deluge

This item shows the torrents of Deluge and how far each one is.

| Setting | Key |
|---|---|
| Switch | `homepageDelugeEnabled` |
| Level | `homepageDelugeAuth` |
| Place | `homepageOrderdeluge` |
| Address | `delugeURL` |
| Password | `delugePassword` |
| Leave out the torrents that are complete and not in the queue | `delugeHideSeeding` |
| Leave out what the tracker said | `delugeHideStatus` |
| Do not check the certificate | `delugeDisableCertCheck` |
| Milliseconds between two loads | `delugeRefresh` |

The address is the address of the web interface of Deluge. The password is the password of the web interface.

PHP Organizr needs the Deluge plugin WebAPI. This version uses the plugin when Deluge has it, and otherwise asks Deluge itself. Without the plugin, Organizr connects the web interface of Deluge to its first daemon when it is not connected yet. To use the plugin:

1. Download the plugin from `https://github.com/idlesign/deluge-webapi/tree/master/dist`.
2. Open the web interface of Deluge. Go to **Preferences**, then **Plugins**, and select **Install plugin**.
3. Switch the plugin **WebAPI** on.

Organizr logs in to Deluge with the password each time it loads the torrents. Deluge has ten seconds to answer.

The setting `delugeHideCompleted` of PHP Organizr has no effect, and the settings page does not show it. PHP Organizr hides a torrent as "completed" only when Deluge sends its progress as the text `100`. Deluge sends a number.

The connection test of Deluge needs the switch of the item, as in PHP Organizr.

## rTorrent

This item shows the torrents of rTorrent and how far each one is.

| Setting | Key |
|---|---|
| Switch | `homepagerTorrentEnabled` |
| Level | `homepagerTorrentAuth` |
| Place | `homepageOrderrTorrent` |
| Address | `rTorrentURL` |
| Second address | `rTorrentURLOverride` |
| User name | `rTorrentUsername` |
| Password | `rTorrentPassword` |
| Leave out the torrents that upload | `rTorrentHideSeeding` |
| Leave out the torrents that are complete and closed | `rTorrentHideCompleted` |
| Sorted by | `rTorrentSortOrder` |
| The most torrents | `rTorrentLimit` |
| Leave out the torrents with these labels | `rTorrentIgnoreLabel` |
| Do not check the certificate | `rTorrentDisableCertCheck` |
| Milliseconds between two loads | `rTorrentRefresh` |

Organizr talks to rTorrent with XML-RPC. It puts `/RPC2` after the address, unless the address holds `.php`. These addresses are possible:

| Address | Organizr asks |
|---|---|
| `http://<host>:8080` | `http://<host>:8080/RPC2` |
| `https://<host>/xmlrpc.php` | `https://<host>/xmlrpc.php` |
| `https://<host>/rutorrent/plugins/httprpc/action.php` | The same address |

Use the second address only when the address does not work. A second address that holds the address stands in place of the address. Another second address stands after the address. In both cases, Organizr adds no `/RPC2`.

Organizr sends the user name and the password only after the web server in front of rTorrent asked for them, and only when both hold a value. It answers a request for Basic or for Digest authentication. With Digest authentication, the password itself does not go to the server.

The label of a torrent is its first custom field, as ruTorrent sets it. A torrent is left out when the setting for the labels holds its label. Capital letters make no difference. A torrent without a label is never left out for its label.

When the limit is 0, the item shows up to 1000 torrents.

## uTorrent

This item shows the torrents of uTorrent and how far each one is.

| Setting | Key |
|---|---|
| Switch | `homepageuTorrentEnabled` |
| Level | `homepageuTorrentAuth` |
| Place | `homepageOrderuTorrent` |
| Address | `uTorrentURL` |
| User name | `uTorrentUsername` |
| Password | `uTorrentPassword` |
| Leave out the torrents that upload | `uTorrentHideSeeding` |
| Leave out the torrents that are finished | `uTorrentHideCompleted` |
| Do not check the certificate | `uTorrentDisableCertCheck` |
| Milliseconds between two loads | `uTorrentRefresh` |

The address is the address of the web interface of uTorrent without `/gui` at its end.

Organizr asks uTorrent for a token and a cookie each time it loads the torrents. It does not store them. The settings `uTorrentToken` and `uTorrentCookie` of PHP Organizr are not used.

## JDownloader

This item shows the packages that JDownloader downloads, and the packages in its link grabber.

| Setting | Key |
|---|---|
| Switch | `homepageJdownloaderEnabled` |
| Level | `homepageJdownloaderAuth` |
| Place | `homepageOrderjdownloader` |
| Address | `jdownloaderURL` |
| User name | `jdownloaderUsername` |
| Password | `jdownloaderPassword` |
| Do not check the certificate | `jdownloaderDisableCertCheck` |
| Milliseconds between two loads | `jdownloaderRefresh` |

Organizr does not talk to JDownloader itself. It reads from a service that does, as PHP Organizr: the Python module `myjd-api`, or FeedCrawler.

| Service | Address |
|---|---|
| `myjd-api` | The address of the module, for example `http://<host>:8080` |
| FeedCrawler | The address of FeedCrawler with `/api/myjd` at its end |

Organizr sends the user name and the password when one of them holds a value.

The item shows the packages. It has no buttons to start, pause or stop the downloads. The buttons of PHP Organizr do nothing.

## Pi-hole

This item shows how many requests Pi-hole got in the last 24 hours, how many it blocked, and which domains it blocked. Pi-hole is a server that blocks advertisements for a whole network.

| Setting | Key |
|---|---|
| Switch | `homepagePiholeEnabled` |
| Level | `homepagePiholeAuth` |
| Place | `homepageOrderPihole` |
| Addresses | `piholeURL` |
| Passwords | `piholeToken` |
| Add the installs up | `homepagePiholeCombine` |
| Show the title | `piholeHeaderToggle` |
| Milliseconds between two loads | `homepagePiholeRefresh` |

The address is the address of Pi-hole without `/admin`, for example `http://192.168.1.2`. The password is the password of the web interface, or an app password from the settings of Pi-hole. Organizr logs in with it each time it loads the item, and removes its older sessions.

For more than one install, put commas between the addresses, and give the passwords in the same order with commas between them. Organizr asks an address only when the list of passwords has a place for it.

The answer names each install by its address.

The item works with Pi-hole 6, as in PHP Organizr 2.1.5000. Pi-hole 5 is not supported.

## Weather and air

This item shows the weather of the next hours, the air quality and the pollen for one place. The data comes from the API of BreezoMeter.

| Setting | Key |
|---|---|
| Switch | `homepageWeatherAndAirEnabled` |
| Level | `homepageWeatherAndAirAuth` |
| Place on the homepage | `homepageOrderWeatherAndAir` |
| Latitude | `homepageWeatherAndAirLatitude` |
| Longitude | `homepageWeatherAndAirLongitude` |
| API key | `breezometerToken` |
| Units: `imperial` or `metric` | `homepageWeatherAndAirUnits` |
| Show the weather | `homepageWeatherAndAirWeatherEnabled` |
| Show the air quality | `homepageWeatherAndAirAirQualityEnabled` |
| Show the pollen | `homepageWeatherAndAirPollenEnabled` |
| Show a title | `homepageWeatherAndAirWeatherHeaderToggle` |
| Title | `homepageWeatherAndAirWeatherHeader` |
| Milliseconds between two loads | `homepageWeatherAndAirRefresh` |

Enter the latitude and the longitude as numbers, for example `52.52` and `13.40`. A map service shows them for an address.

The item needs an API key of BreezoMeter. PHP Organizr comes with a key of its author. This application comes with no key. An import brings the key that your PHP install used.

A kind of data that the API refuses is not shown. If the API gives nothing at all, the item says so.

PHP Organizr can search the latitude and the longitude of a city. That search uses a key of the author of PHP Organizr at a map service, and the settings have no place for a key of your own. So this application has no such search. Enter the two numbers yourself.

## Tautulli

This item shows statistics of your Plex server from Tautulli: the films and series that are watched most, the most active users and players, and a table of the libraries.

| Setting | Key |
|---|---|
| Switch | `homepageTautulliEnabled` |
| Level | `homepageTautulliAuth` |
| Place | `homepageOrdertautulli` |
| Address of Tautulli | `tautulliURL` |
| API key | `tautulliApikey` |
| Do not check the certificate | `tautulliDisableCertCheck` |
| Show a title | `tautulliHeaderToggle` |
| Title | `tautulliHeader` |
| Milliseconds between two loads | `homepageTautulliRefresh` |

The address of Tautulli is the one of [single sign-on](single-sign-on.md), on the tab **Login** of the settings. If that setting holds more than one address, the homepage shows the first install.

The statistics are in three groups. Each group has a level of its own, so you can show a user what is watched and keep back who watches.

| Group | Level | Switches |
|---|---|---|
| What is watched | `homepageTautulliViewsAuth` | `tautulliTopMovies`, `tautulliPopularMovies`, `tautulliTopTV`, `tautulliPopularTV` |
| Users and players | `homepageTautulliMiscAuth` | `tautulliTopUsers`, `tautulliTopPlatforms` |
| Libraries | `homepageTautulliLibraryAuth` | `tautulliLibraries` |

A person without the level of a group does not get the data of that group from the server.

To leave libraries out, enter their ids in Tautulli with commas between them: `homepageTautulliViewingStatsExclude` for what is watched, and `homepageTautulliLibraryStatsExclude` for the table.

The poster of the first row of a statistic comes through the picture cache of the server, as the [pictures of events](#pictures-of-events) do.

## Plex

Three items show your Plex server: what plays now, what was added last, and the video playlists.

All three need these settings:

| Setting | Key |
|---|---|
| Switch for Plex on the homepage | `homepagePlexEnabled` |
| Level for Plex on the homepage | `homepagePlexAuth` |
| Address of the Plex server | `plexURL` |
| Plex token | `plexToken` |
| Id of the Plex server | `plexID` |
| Do not check the certificate | `plexDisableCertCheck` |

The token and the id are the ones of [the login through Plex](plex-login.md), on the tab **Login**. The address is the address of the Plex server itself, for example `http://192.168.1.10:32400`.

Each item has a switch, a level and a place of its own:

| Item | Switch | Level | Place |
|---|---|---|---|
| Playing now | `homepagePlexStreams` | `homepagePlexStreamsAuth` | `homepageOrderplexnowplaying` |
| New items | `homepagePlexRecent` | `homepagePlexRecentAuth` | `homepageOrderplexrecent` |
| Playlists | `homepagePlexPlaylist` | `homepagePlexPlaylistAuth` | `homepageOrderplexplaylist` |

A person needs the level of Plex on the homepage and the level of the item.

### Playing now

Each stream shows its picture, how much was watched, and how it reaches its player.

Who plays is shown only when you switch it on:

| Setting | Key |
|---|---|
| Show who plays, and from which address | `homepageShowStreamNames`, with the level `homepageShowStreamNamesAuth` |
| Show who plays, without the address | `homepageShowStreamNamesWithoutIp`, with the level `homepageShowStreamNamesWithoutIpAuth` |
| Show the names that Tautulli has for the users | `homepageUseCustomStreamNames` |

A person without these levels does not get the names and the addresses from the server.

`homepagePlexStreamsExclude` leaves libraries out. Enter the ids of the libraries in Plex with commas between them.

### New items and playlists

The new items are the films, the series and the music that were added last, the newest first. `homepageRecentLimit` is the largest number for each of the three kinds. `homepagePlexRecentExclude` leaves libraries out.

The playlists are the video playlists of the owner of the token. A playlist with the word "private" in its title is not shown.

A poster opens the details of its item, with a link to the item in Plex Web. The link goes to `app.plex.tv`, or to the address in `plexTabURL` when that setting has one.

### Pictures

The server fetches the pictures of the Plex server in the size that the homepage shows, and keeps each one for a week in the folder `cache` of the data folder. `cacheImageSize` sets the size.

A person needs the level of Plex on the homepage to load a picture.

### Not built yet

PHP Organizr has a search of the Plex server in its top bar. The server of this application has the route for it. The interface does not show the search yet.

## Emby

Two items show your Emby server: what plays now, and what was added last.

Both need these settings:

| Setting | Key |
|---|---|
| Switch for Emby on the homepage | `homepageEmbyEnabled` |
| Level for Emby on the homepage | `homepageEmbyAuth` |
| Address of the Emby server | `embyURL` |
| API key | `embyToken` |
| Do not check the certificate | `embyDisableCertCheck` |
| Address of an item in the web interface | `homepageEmbyLink` |
| Name of the tab of Organizr that shows Emby | `embyTabName` |
| Address of Emby from outside your network | `embyTabURL` |

The address is the address of the Emby server itself, for example `http://192.168.1.10:8096`. You make the API key in the dashboard of Emby. Organizr sends the key in a header of each request, so the key is in no address and in no log.

Each item has a switch, a level and a place of its own:

| Item | Switch | Level | Place |
|---|---|---|---|
| Playing now | `homepageEmbyStreams` | `homepageEmbyStreamsAuth` | `homepageOrderembynowplaying` |
| New items | `homepageEmbyRecent` | `homepageEmbyRecentAuth` | `homepageOrderembyrecent` |

A person needs the level of Emby on the homepage and the level of the item.

These settings are the same ones that Plex uses. You find them in the groups of Plex:

| Setting | Key |
|---|---|
| Show who plays, and from which address | `homepageShowStreamNames`, with the level `homepageShowStreamNamesAuth` |
| Milliseconds between two loads of what plays | `homepageStreamRefresh` |
| Most new items | `homepageRecentLimit` |
| Milliseconds between two loads of the new items | `homepageRecentRefresh` |
| Size of the pictures | `cacheImageSize` |

### Playing now

Each stream shows its picture, how much was watched, and how it reaches its player. A player that plays nothing is not shown.

A person without the level of `homepageShowStreamNames` does not get the names and the addresses of the players from the server.

### New items

The new items are the items that were added last and that one user of the Emby server did not play. The list holds series, episodes, albums, songs, films and videos.

Organizr looks for the user of the Emby server who has the same name as the person in Organizr. Capital letters make no difference. If there is no such user, Organizr takes an administrator of the Emby server.

A poster opens the details of its item, with a link to the item in the web interface of Emby. `homepageEmbyLink` holds the address of that link. Organizr puts the id of the item in the place of `{id}`, and the id of the server in the place of `{serverId}`. If the setting is empty, the details show no link.

### Pictures

The server fetches the pictures of the Emby server in the size that the homepage shows, and keeps each one for a week in the folder `cache` of the data folder. `cacheImageSize` sets the size.

A person needs the level of Emby on the homepage to load a picture.

### Test the connection

The test asks the Emby server for the list of its users. The test needs the switch of Emby on the homepage, as in PHP Organizr.

## Jellyfin

Two items show your Jellyfin server: what plays now, and what was added last.

Both need these settings:

| Setting | Key |
|---|---|
| Switch for Jellyfin on the homepage | `homepageJellyfinEnabled` |
| Level for Jellyfin on the homepage | `homepageJellyfinAuth` |
| Address of the Jellyfin server | `jellyfinURL` |
| API key | `jellyfinToken` |
| Do not check the certificate | `jellyfinDisableCertCheck` |
| Address of an item in the web interface | `homepageJellyfinLink` |
| Name of the tab of Organizr that shows Jellyfin | `jellyfinTabName` |

The address is the address of the Jellyfin server itself, for example `http://192.168.1.10:8096`. You make the API key in the dashboard of Jellyfin. Organizr sends the key in a header of each request, so the key is in no address and in no log.

Each item has a switch, a level and a place of its own:

| Item | Switch | Level | Place |
|---|---|---|---|
| Playing now | `homepageJellyfinStreams` | `homepageJellyStreamsAuth` | `homepageOrderjellyfinnowplaying` |
| New items | `homepageJellyfinRecent` | `homepageJellyfinRecentAuth` | `homepageOrderjellyfinrecent` |

A person needs the level of Jellyfin on the homepage and the level of the item.

The key of the level of the streams has the short name `homepageJellyStreamsAuth`, as in PHP Organizr.

These settings are the same ones that Plex uses. You find them in the groups of Plex:

| Setting | Key |
|---|---|
| Show who plays, and from which address | `homepageShowStreamNames`, with the level `homepageShowStreamNamesAuth` |
| Milliseconds between two loads of what plays | `homepageStreamRefresh` |
| Most new items | `homepageRecentLimit` |
| Milliseconds between two loads of the new items | `homepageRecentRefresh` |
| Size of the pictures | `cacheImageSize` |

### Playing now

Each stream shows its picture, how much was watched, and how it reaches its player. A player that plays nothing is not shown.

A person without the level of `homepageShowStreamNames` does not get the names and the addresses of the players from the server.

### New items

The new items are the items that were added last and that one user of the Jellyfin server did not play.

Organizr looks for the user of the Jellyfin server who has the same name as the person in Organizr. Capital letters make no difference. If there is no such user, Organizr takes an administrator of the Jellyfin server.

A poster opens the details of its item, with a link to the item in the web interface of Jellyfin. `homepageJellyfinLink` holds the address of that link. Organizr puts the id of the item in the place of `{id}`, and the id of the server in the place of `{serverId}`. If the setting is empty, the details show no link.

### Pictures

The server fetches the pictures of the Jellyfin server in the size that the homepage shows, and keeps each one for a week in the folder `cache` of the data folder. `cacheImageSize` sets the size.

A person needs the level of Jellyfin on the homepage to load a picture.

### Test the connection

The test asks the Jellyfin server for the list of its users. The test works with the switch of Jellyfin on the homepage off.

## Overseerr

This item shows the requests for films and series in Overseerr, the newest first.

| Setting | Key |
|---|---|
| Switch | `homepageOverseerrEnabled` |
| Level | `homepageOverseerrAuth` |
| Place | `homepageOrderoverseerr` |
| Address of Overseerr | `overseerrURL` |
| API key | `overseerrToken` |
| Do not check the certificate | `overseerrDisableCertCheck` |
| Show a user only the own requests | `overseerrLimitUser` |
| Most requests that are loaded | `overseerrLimit` |
| Milliseconds between two loads | `overseerrRefresh` |

The address of Overseerr is the one of [single sign-on](single-sign-on.md), on the tab **Login** of the settings.

With `overseerrLimitUser`, a user sees the requests that the user made, and the requests of the fallback user of single sign-on. Organizr compares the user name in Organizr with the user name in Overseerr. An Admin and a Co-Admin see all requests.

A request opens its details. There, an Admin or a Co-Admin can approve, deny and delete the request, and mark it as available or not available. PHP Organizr lets every person who sees the requests do that.

### New requests

The server can make a new request for a person:

| Setting | Key |
|---|---|
| Level for a new request | `homepageOverseerrRequestAuth` |
| Seasons of a new request for a series | `overseerrTvDefault`: `all`, `first`, `last` or `user` |
| Send new requests to the 4K service | `overseerrPrefer4K` |

The request is made as the person, with the session of Overseerr that single sign-on gave the browser. So a person needs single sign-on for Overseerr to make a request.

The interface of this application does not have the search for a film or a series to request yet. Until then, make new requests in Overseerr.

## Ombi

This item shows the requests for films and series in Ombi, the newest first.

| Setting | Key |
|---|---|
| Switch | `homepageOmbiEnabled` |
| Level | `homepageOmbiAuth` |
| Place | `homepageOrderombi` |
| Address of Ombi | `ombiURL` |
| API key | `ombiToken` |
| Do not check the certificate | `ombiDisableCertCheck` |
| Show a user only the own requests | `ombiLimitUser` |
| Most requests that are loaded | `ombiLimit` |
| Show the alias of a user in place of the user name | `ombiAlias` |
| Milliseconds between two loads | `ombiRefresh` |

The address of Ombi is the one of [single sign-on](single-sign-on.md), on the tab **Login** of the settings.

With `ombiLimitUser`, a user sees the requests that the user made, and the requests of the fallback user of single sign-on. Organizr compares the user name in Organizr with the user name in Ombi. An Admin and a Co-Admin see all requests.

Five switches say which requests show when the homepage opens. A person can switch each state on and off on the homepage.

| Switch | Key |
|---|---|
| The available requests | `ombiDefaultFilterAvailable` |
| The requests that are not available | `ombiDefaultFilterUnavailable` |
| The approved requests | `ombiDefaultFilterApproved` |
| The waiting requests | `ombiDefaultFilterUnapproved` |
| The denied requests | `ombiDefaultFilterDenied` |

A request opens its details. There, an Admin or a Co-Admin can approve, deny and delete the request, and mark it as available or not available. PHP Organizr lets every person who sees the requests do that.

A script gets the requests with `GET /api/v2/homepage/ombi/requests`. The path can name the type, the largest number of requests and the number of requests to skip: `GET /api/v2/homepage/ombi/requests/movie/10/20`. The type is `movie`, `tv` or `both`.

### New requests in Ombi

The server can make a new request for a person, with `POST /api/v2/homepage/ombi/requests/{type}/{id}`. The type is `movie` or `tv`, and the id is the id of the film or the series in The Movie Database.

| Setting | Key |
|---|---|
| Level for a new request | `homepageOmbiRequestAuth` |
| Seasons of a new request for a series | `ombiTvDefault`: `all`, `first` or `last` |

The request is made as the person, with the token of Ombi that single sign-on gave the browser. So a person needs single sign-on for Ombi to make a request.

Before the request, Organizr asks Ombi whether the film or the series is available or requested already. For a series, Ombi also names the id of the series in TheTVDB, which the request needs.

The interface of this application does not have the search for a film or a series to request yet. Until then, make new requests in Ombi.

## Jackett

This item searches the indexers of [Jackett](https://github.com/Jackett/Jackett) for a text, and shows the releases that Jackett finds in a table.

| Setting | Key |
|---|---|
| Switch | `homepageJackettEnabled` |
| Level | `homepageJackettAuth` |
| Place | `homepageOrderJackett` |
| Address of Jackett | `jackettURL` |
| API key | `jackettToken` |
| Do not check the certificate | `jackettDisableCertCheck` |
| Download with the black hole folder | `homepageJackettBackholeDownload` |

Type a text and select **Search**. The table shows the date, the tracker, the name, the size, the files, the grabs, the seeders and the leechers of each release. The newest release stands first. The name of a release opens its page at the tracker.

The last column has the download:

- With **Download with the black hole folder** on, a release has a button. The button makes Jackett put the torrent file into its black hole folder. Set this folder in Jackett, and let your download client watch it.
- Without the switch, a release has a link: its magnet link, or its page at the tracker.

The API key of Jackett stays on the server. Jackett puts its key into its download links. Organizr cuts the key out of the answer before the answer goes to the browser, and puts the key back when it asks Jackett for a black hole download. So the download link of Jackett itself does not work from the browser, and the interface does not show it.

A person who has the level of the item can search and can start a black hole download.

## Prowlarr

This item searches the indexers of [Prowlarr](https://prowlarr.com/) for a text, and shows the releases that Prowlarr finds in a table.

| Setting | Key |
|---|---|
| Switch | `homepageProwlarrEnabled` |
| Level | `homepageProwlarrAuth` |
| Place | `homepageOrderProwlarr` |
| Address of Prowlarr | `ProwlarrURL` |
| API key | `ProwlarrToken` |
| Do not check the certificate | `ProwlarrDisableCertCheck` |
| Download with the download client of Prowlarr | `homepageProwlarrBackholeDownload` |

The keys of the address, the API key and the certificate switch start with a capital letter. These are the names in the default settings of PHP Organizr.

The table shows the date, the indexer, the title, the size, the grabs, the seeders and the leechers of each release. The release with the most seeders stands first.

The last column has the download:

- With **Download with the download client of Prowlarr** on, a release has a button. The button makes Prowlarr send the release to the download client that you set up in Prowlarr.
- Without the switch, a release has a link to its page at the indexer, or its magnet link when the indexer gives one. The download link of Prowlarr holds the API key of Prowlarr, so the interface does not show it.

The API key goes to Prowlarr in a header, and it stays on the server, as for Jackett.

## AdGuard Home

This item shows how many requests [AdGuard Home](https://adguard.com/adguard-home/overview.html) got, how many it blocked, how long it needed for an answer, and how many domains are on its block lists.

| Setting | Key |
|---|---|
| Switch | `homepageAdGuardEnabled` |
| Level | `homepageAdGuardAuth` |
| Place | `homepageOrderAdguard` |
| Addresses | `adguardURL` |
| User name | `adGuardUsername` |
| Password | `adGuardPassword` |
| Show the title | `adguardHeaderToggle` |
| Show the installs in one row of cards | `homepageAdGuardCombine` |
| Show the total of the queries | `adguardQueriesToggle` |
| Show the blocked queries | `adguardQueriesBlockedToggle` |
| Show the part that was blocked | `adguardPercentToggle` |
| Show the processing time | `adguardProcessingToggle` |
| Show the domains on the block lists | `adguardDomainListToggle` |
| Milliseconds between two loads | `homepageAdGuardRefresh` |

For more than one install, put commas between the addresses. The answer names each install by its host.

The user name and the password are those of the web interface of AdGuard Home. Organizr sends them to each install, as PHP Organizr does. Without a user name, Organizr sends no login, which works only with an install that has no login. An install that refuses the login shows no numbers, and the connection test names it with "Unknown Failure".

The number of domains on the block lists is the sum of the rules of all filter lists and of the rules of the install itself.

## Netdata

This item shows up to seven charts. Each chart shows one number of an install of [Netdata](https://www.netdata.cloud/), as a ring or as a gauge.

| Setting | Key |
|---|---|
| Switch | `homepageNetdataEnabled` |
| Level | `homepageNetdataAuth` |
| Place | `homepageOrderNetdata` |
| Address of Netdata | `netdataURL` |
| Do not check the certificate | `netdataDisableCertCheck` |
| Custom charts | `netdataCustom` |
| Milliseconds between two loads | `homepageNetdataRefresh` |

Each chart has these settings, where `<N>` is the number of the chart from 1 to 7:

| Setting | Key | Values |
|---|---|---|
| Switch | `netdata<N>Enabled` | |
| Title | `netdata<N>Title` | |
| What the chart shows | `netdata<N>Data` | `disk-read`, `disk-write`, `cpu`, `net-in`, `net-out`, `ram-used`, `swap-used`, `disk-used`, `disk-avail`, `custom` |
| Form | `netdata<N>Chart` | `easypiechart` for a ring, `gauge` for a gauge |
| Colour | `netdata<N>Colour` | A hexadecimal colour without the number sign, for example `46e302` |
| Size | `netdata<N>Size` | `lg`, `md`, `sm` |
| Show on large, medium and small screens | `netdata<N>lg`, `netdata<N>md`, `netdata<N>sm` | |

The disk and the network charts show megabytes or megabits in a second, and their part of the highest value that Netdata has. The other charts show a part of 100.

When Netdata does not answer, the charts show without a number.

### Custom charts

A chart with the value `custom` reads its number from a request that you write. The setting `netdataCustom` holds JSON with one entry for each custom chart, by the number of the chart:

```json
{
  "1": {
    "url": "api/v1/data?chart=system.ram&format=array&points=540&group=average&gtime=0&options=absolute|percentage|jsonwrap|nonzero&after=-540&dimensions=used|buffers|active|wired",
    "value": "result,0",
    "units": "%",
    "max": 100
  }
}
```

| Field | Meaning | Needed |
|---|---|---|
| `url` | The path of the request, after the address of Netdata. | Yes |
| `value` | The way to the number in the JSON answer: keys and places, with commas between them. | Yes |
| `units` | The unit that the chart shows. The default is `%`. | No |
| `max` | The highest value. The default is 100. | No |
| `mutator` | Calculations on the number, with commas between them, for example `/1000,*8`. The signs are `+`, `-`, `*` and `/`. | No |
| `netdata` | The address of another install of Netdata to ask. | No |

A chart whose entry is missing or not complete shows the reason and no number.

## Monitorr

This item shows the services that an install of [Monitorr](https://github.com/Monitorr/Monitorr) watches, and whether each one is online.

| Setting | Key |
|---|---|
| Switch | `homepageMonitorrEnabled` |
| Level | `homepageMonitorrAuth` |
| Place | `homepageOrderMonitorr` |
| Address of Monitorr | `monitorrURL` |
| Do not check the certificate | `monitorrDisableCertCheck` |
| Show a title | `monitorrHeaderToggle` |
| Title | `monitorrHeader` |
| Small form | `monitorrCompact` |
| Milliseconds between two loads | `homepageMonitorrRefresh` |

Monitorr has no API. Organizr reads the HTML that the page of Monitorr loads from `assets/php/loop.php`, and takes the name, the state, the picture and the link of each service from it. Another version of Monitorr can write other HTML, and the item then shows no services.

The services that are offline stand first. A service is online, offline or unresponsive.

Organizr loads the picture of each service from Monitorr and sends it with the answer. A picture of more than 512 KB is left out.

Organizr sends no key to Monitorr. PHP Organizr sends its own API key with each request.

## Speedtest Tracker

This item shows the last result of an install of [Speedtest Tracker](https://github.com/henrywhitaker3/Speedtest-Tracker): the ping, the download speed and the upload speed. It also shows the average, the highest and the lowest values when the install sends them.

| Setting | Key |
|---|---|
| Switch | `homepageSpeedtestEnabled` |
| Level | `homepageSpeedtestAuth` |
| Place | `homepageOrderSpeedtest` |
| Address of Speedtest Tracker | `speedtestURL` |
| Do not check the certificate | `speedtestDisableCertCheck` |
| Show a title | `speedtestHeaderToggle` |
| Title | `speedtestHeader` |
| Milliseconds between two loads | `homepageSpeedtestRefresh` |

Organizr reads `api/speedtest/latest` of the install, without a key.

This item is not the [speed test plugin](speed-test-plugin.md). The plugin measures the connection between a browser and Organizr. This item shows what another service measured.

## OctoPrint

This item shows the state of a 3D printer with [OctoPrint](https://octoprint.org/). While the printer prints, it also shows the file, the progress, the total print time and the time that is left. When the webcam of OctoPrint is on, the item shows its stream.

| Setting | Key |
|---|---|
| Switch | `homepageOctoprintEnabled` |
| Level | `homepageOctoprintAuth` |
| Place | `homepageOrderOctoprint` |
| Address of OctoPrint | `octoprintURL` |
| API key | `octoprintToken` |
| Do not check the certificate | `octoprintDisableCertCheck` |
| Show a title | `octoprintHeaderToggle` |
| Title | `octoprintHeader` |
| Milliseconds between two loads | `homepageOctoprintRefresh` |

Make the API key in the settings of OctoPrint, under **Application Keys**.

Organizr reads the print job and the settings of OctoPrint. Of the settings, the answer holds only whether the webcam is on and the address of its stream. The other settings of OctoPrint hold keys and passwords, and they stay on the server.

The browser loads the stream of the webcam from OctoPrint. So the browser must reach the address of OctoPrint.

## UniFi

This item shows the parts of a network from a UniFi Network controller: the wireless network, the wired network, the internet connection and the VPN. Each part that is in order has a card with its clients and its traffic.

| Setting | Key |
|---|---|
| Switch | `homepageUnifiEnabled` |
| Level | `homepageUnifiAuth` |
| Place | `homepageOrderunifi` |
| Address of the controller | `unifiURL` |
| User name | `unifiUsername` |
| Password | `unifiPassword` |
| Name of the site | `unifiSiteName` |
| Do not check the certificate | `unifiDisableCertCheck` |
| Milliseconds between two loads | `homepageUnifiRefresh` |

Use a local account of the controller without a second factor. Capital and small letters count in the user name.

There are two kinds of controller, and Organizr finds out which one it talks to:

| Controller | How Organizr knows it | Site |
|---|---|---|
| A console with UniFi OS, for example a Dream Machine or a Cloud Key of the second generation | The start page sends a header `x-csrf-token`. | Always the site `default`. |
| The classic controller, for example the software on a computer | The start page sends no such header. | The site of the setting `unifiSiteName`. |

For the classic controller, the name of the site is its short name, not its description. The first site has the name `default`. To see the names, save the address and the login, and select **Read the sites of the controller**.

Organizr logs in for each load, uses the cookie of the login for one request, and does not store it. The setting `unifiCookie` is not used.

The controller of most installs has a certificate that no authority signed. So the switch **Do not check the certificate** is on in the default settings.

## PromPage

This item shows the services of a [PromPage](https://github.com/henrywhitaker3/prompage) status page, and whether each one is up.

| Setting | Key |
|---|---|
| Switch | `homepagePromPageEnabled` |
| Place | `homepageOrderPromPage` |
| Address of PromPage | `promPageURL` |
| Show a title | `homepagePromPageHeaderToggle` |
| Title | `homepagePromPageHeader` |
| Small form | `homepagePromPageCompact` |
| Show the uptime | `homepagePromPageShowUptime` |
| Milliseconds between two loads | `homepagePromPageRefresh` |

The item has no level. Every person who opens the homepage sees it when its switch is on, also without a login. PHP Organizr does the same.

Organizr reads `api/services` of PromPage. That address needs no key.

## Emby live TV

This item shows the live television of the Emby server: the number of the timers and of the recordings, the recordings to come, and the recordings that are made.

| Setting | Key |
|---|---|
| Switch | `homepageEmbyLiveTVTrackerEnabled` |
| Level | `homepageEmbyLiveTVTrackerAuth` |
| Place | `homepageOrderEmbyLiveTVTracker` |
| Minutes between two loads | `homepageEmbyLiveTVTrackerRefresh` |
| Show the numbers of the timers only | `homepageEmbyLiveTVTrackerCompactView` |
| Show the length of a recording | `homepageEmbyLiveTVTrackerShowDuration` |
| Show the series and the title of the episode | `homepageEmbyLiveTVTrackerShowSeriesInfo` |
| Show the user who set a timer | `homepageEmbyLiveTVTrackerShowUserInfo` |
| Largest number of scheduled recordings | `homepageEmbyLiveTVTrackerMaxItems` |
| Show the recordings that are made | `homepageEmbyLiveTVTrackerShowCompleted` |
| Largest number of recordings that are made | `homepageEmbyLiveTVTrackerMaxCompletedItems` |

The item uses the address and the API key of [Emby](#emby): `embyURL` and `embyToken`. It does not need the switch of the Emby item.

The time between two loads is in minutes. Most other items have it in milliseconds.

The button **Test the connection** shows the name of the server, and whether the server has live television. A script runs the test with `POST /api/v2/test/embyLiveTVTracker`.

If the server does not answer a request, the numbers of that request are 0 and its list is empty. The item shows no error then. PHP Organizr does the same.

## JellyStat

This item shows the statistics that [JellyStat](https://github.com/CyferShepard/Jellystat) keeps for an Emby or a Jellyfin server: the libraries, the users, and the films, the series and the music that were played most. It can also show the pages of JellyStat in a frame.

| Setting | Key |
|---|---|
| Switch | `homepageJellyStatEnabled` |
| Level | `homepageJellyStatAuth` |
| Place | `homepageOrderJellyStat` |
| What the item shows: `native` for the statistics, `iframe` for the frame | `homepageJellyStatDisplayMode` |
| Address of JellyStat | `jellyStatURL` |
| Address of JellyStat for the server | `jellyStatInternalURL` |
| API key | `jellyStatApikey` |
| Do not check the certificate | `jellyStatDisableCertCheck` |
| Minutes between two loads | `homepageJellyStatRefresh` |
| Days that the statistics look back | `homepageJellyStatDays` |
| Show the libraries | `homepageJellyStatShowLibraries` |
| Show the users | `homepageJellyStatShowUsers` |
| Largest number of users in the list | `homepageJellyStatMaxItems` |
| Show what was played most | `homepageJellyStatShowMostWatched` |
| Show the films, the series, the music | `homepageJellyStatShowMostWatchedMovies`, `homepageJellyStatShowMostWatchedShows`, `homepageJellyStatShowMostListenedMusic` |
| Largest number of items in each of these lists | `homepageJellyStatMostWatchedCount` |
| Height of the frame in pixels | `homepageJellyStatIframeHeight` |
| Let the frame scroll | `homepageJellyStatIframeScrolling` |

Make the API key in the settings of JellyStat. The statistics need the key. The frame does not.

- The browser loads the posters and the frame from the address of JellyStat. So that address must be one that the browser can reach.
- If the server reaches JellyStat at another address, enter that one as the address for the server.
- The API key goes to JellyStat in a header. PHP Organizr puts it into the address.
- JellyStat counts a play without a video, or of less than ten minutes, as music. PHP Organizr reads the history in the same way.

A poster opens the details of its item. Organizr asks the media server for them when the homepage shows that server: Jellyfin first, or else Emby. The pictures of the details then come through Organizr, so a person needs the level of that server to see them. Without such a server, Organizr asks JellyStat.

The frame shows the pages of JellyStat as they are. JellyStat must allow other sites to show it in a frame, and a person logs in at JellyStat in the frame.

A script gets the statistics with `GET /api/v2/homepage/jellystat`, and the details of an item with `POST /api/v2/homepage/jellystat/metadata` and the id of the item as `key`.

## Donate

This item lets a person donate money to you. The payments go to your own [Stripe](https://stripe.com/) account. Organizr sees no card: a person pays on a page of Stripe.

| Setting | Key |
|---|---|
| Switch | `homepageDonateEnabled` |
| Level | `homepageDonateAuth` |
| Place | `homepageOrderDonate` |
| Publishable key of Stripe | `homepageDonatePublicToken` |
| Secret key of Stripe | `homepageDonateSecretToken` |
| Id of the product | `homepageDonateProductID` |
| Title of the item | `homepageDonateCustomizeHeading` |
| Text of the item | `homepageDonateCustomizeDescription` |
| Smallest amount, in cents | `homepageDonateMinimum` |
| Let a person see the own donations | `homepageDonateShowUserHistory` |

To set the item up:

1. Log in at Stripe, or make an account.
2. Add a product with a one-time price. Copy the id of the product.
3. Open the page **API keys** of Stripe. Copy the publishable key and the secret key.
4. Enter the three values in Organizr, and select **Save**.

A person enters an amount in whole US dollars and selects **Donate**. A new window opens the page of Stripe. After the payment, Stripe sends the window back to Organizr.

- The amount is at least the smallest amount of the settings.
- Only a person with the level of the item can start a payment. In PHP Organizr, everybody can, also when the item is off.
- The item shows its text as text. HTML in the text is not read.
- With **Let a person see the own donations**, a person sees the payments with the e-mail address of the own account. An Admin sees the payments of everybody, with the e-mail addresses.
- The secret key is a secret of the settings. No answer of Organizr holds it.

Use a test key of Stripe first. Stripe then takes no money, and you can pay with a test card.

## Routes without an item

PHP Organizr has three more routes that help its pages. This application has them for scripts. Its own pages do not use them.

| Route | What it does | Who can use it |
|---|---|---|
| `GET /api/v2/homepage/youtube/{title}` | Searches YouTube for the trailer of a title, with the API key of the setting `youtubeAPI`. | Everybody |
| `POST /api/v2/homepage/scrape` | Fetches the page at the address `url` of the body. | An Admin or a Co-Admin |
| `POST /api/v2/homepage/weather/coordinates` | Searches the latitude and the longitude of a place. It finds nothing here. | An Admin or a Co-Admin |

- The trailer search needs your own API key of YouTube. PHP Organizr falls back on keys of its author. This application does not, so the answer is an error without a key. Enter your key in the group **YouTube** of the settings of the homepage.
- The fetch of a page takes an address of the wide web only. It does not fetch an address of the server itself or of a private network, it follows no redirect, and a page has 2 MB at most. In PHP Organizr, everybody can use the route without a login.
