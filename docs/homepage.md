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
- [Calendar](#calendar), with Sonarr, Radarr and Lidarr
- [Downloads of Sonarr and Radarr](#downloads-of-sonarr-and-radarr)
- [SABnzbd](#sabnzbd)
- [qBittorrent](#qbittorrent)
- [Weather and air](#weather-and-air)
- [Plex](#plex): what plays now, new items and playlists
- [Tautulli](#tautulli)

You set all items up in **Settings**, on the tab **Homepage**.

The homepage loads the data of an item again after the time that the item has in its settings. The shortest time is five seconds.

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

### The frame of an item

The homepage shows each custom HTML item in a frame of its own. The browser treats the content of the frame as a page of no site. Scripts of the item run. They cannot read the cookies of Organizr, and they cannot use Organizr as the person who looks at the page.

PHP Organizr writes the HTML into its own page. There, a script of an item can do all that the person can do, for example change settings when an Admin looks at the homepage.

What this means for the HTML of an item:

- The item can use styles and scripts of its own, and it can load files from other sites.
- The item can use the classes and the icons of the style sheet of this application.
- A link without a `target` changes the address of Organizr. The link `<a href="#Plex">` opens the tab "Plex".
- A link with `target="_blank"` opens a new window.
- The frame is as high as its content.

### HTML from PHP Organizr

An import brings the HTML of your items as it is. Look at each item after the import:

- The CSS classes of the PHP interface do not exist here. An item that uses them looks different.
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

The calendar shows the episodes of Sonarr, the films of Radarr and the albums of Lidarr for the days around today.

The calendar has no switch of its own. It shows when a person can see one of the three applications at least. Each application has these settings, here with the keys of Sonarr:

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
| View at the start: month or list | `calendarDefault` |
| First day of a week | `calendarFirstDay` |
| Days before today that are loaded | `calendarStart` |
| Days after today that are loaded | `calendarEnd` |
| Most events that a day of the month shows | `calendarLimit` |
| Milliseconds between two loads | `calendarRefresh` |

An event has a colour for its state:

| State | Meaning |
|---|---|
| Downloaded | The file is there. |
| Not released yet | The release is in the future. |
| Premiere | The first episode of a season, which was not aired yet. |
| Not monitored | An episode that was not aired yet, and that Sonarr does not monitor. |
| Missing | The release is in the past, and there is no file. |

The day of a film is a day of the time zone of the server. Set the environment variable `TZ` of the container to your time zone, for example `TZ=Europe/Berlin`.

### Pictures of events

An event shows the fan art of its series or film. When only Sonarr or Radarr has the picture, a browser cannot load it without the API key. The server then fetches the picture, keeps it in the folder `cache` of the data folder, and sends it from `data/cache/`. The first time, the event shows without its picture for a moment.

You can delete the folder `cache` at any time. The server fetches the pictures again.

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
