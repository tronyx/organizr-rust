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
- [Calendar](#calendar), with Sonarr, Radarr, Lidarr, SickRage, CouchPotato and calendars in the iCal format
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

You set all items up in **Settings**, on the tab **Homepage**.

The homepage loads the data of an item again after the time that the item has in its settings. The shortest time is five seconds.


## Test the connection of a service

The settings of these services have a button **Test the connection**: Plex, Emby, Jellyfin, Tautulli, Overseerr, Ombi, Sonarr, Radarr, Lidarr, SickRage, SABnzbd, qBittorrent, NZBGet, Transmission, Deluge, rTorrent, uTorrent, JDownloader and Pi-hole.

1. Enter the address and the key or the login of the service.
2. Select **Save**. The test reads the settings that are stored.
3. Select **Test the connection**. The page shows whether the service answered, or what is wrong.

Only an Admin or a Co-Admin can run a test. A script can run it with `POST /api/v2/test/<service>`, where the service is the name in lower case without the hyphen, for example `pihole`.

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

The calendar shows the episodes of Sonarr and SickRage, the films of Radarr and CouchPotato, the albums of Lidarr, and the events of calendars in the iCal format, for the days around today.

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

Deluge needs the plugin WebAPI, as in PHP Organizr:

1. Download the plugin from `https://github.com/idlesign/deluge-webapi/tree/master/dist`.
2. Open the web interface of Deluge. Go to **Preferences**, then **Plugins**, and select **Install plugin**.
3. Switch the plugin **WebAPI** on.

Without the plugin, Deluge answers with an error, and the item shows that error.

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

This item shows how many requests Pi-hole got today, how many it blocked, and how many domains are on its lists. Pi-hole is a server that blocks advertisements for a whole network.

| Setting | Key |
|---|---|
| Switch | `homepagePiholeEnabled` |
| Level | `homepagePiholeAuth` |
| Place | `homepageOrderPihole` |
| Addresses | `piholeURL` |
| Tokens | `piholeToken` |
| Add the installs up | `homepagePiholeCombine` |
| Show the title | `piholeHeaderToggle` |
| Milliseconds between two loads | `homepagePiholeRefresh` |

The address is the address of the web interface, for example `http://192.168.1.2/admin`. The token is the API token from the settings of Pi-hole.

For more than one install, put commas between the addresses, and give the tokens in the same order with commas between them. Organizr asks an address only when the list of tokens has a place for it. For two installs without tokens, enter one comma as the tokens.

The answer names each install by its host. Two installs on one host with different ports do not work together, as in PHP Organizr.

The item works with Pi-hole 5. Pi-hole 6 has another API, which this item does not know. PHP Organizr does not know it either.

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

PHP Organizr can search the latitude and the longitude of a city. This application does not have that search yet.

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
