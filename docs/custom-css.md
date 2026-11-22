# Custom CSS

This version of Organizr builds its pages differently from PHP Organizr. Custom CSS written for PHP aims at PHP's ids and classes, such as `#allHealthChecks` or `.white-box`, and those names are not on these pages. This page lists the names that you can use instead, and shows how to change a PHP rule into a rule for this version.

You enter custom CSS in **Settings**, **Customize**, **Appearance**, **Custom CSS**. [Appearance](appearance.md) tells you how the page loads it.

## The names you can rely on

The CSS variables and classes on this page are stable: a release does not change them without a note in the release notes. The tests of the project fail if the interface stops using one of them, or if custom CSS stops reaching them.

The pages have other classes too, for example the classes of Tailwind such as `flex` or `px-4`. Do not aim at those. They can change in any release.

## Colours: CSS variables

Most custom CSS only changes colours. A CSS variable changes a colour everywhere it is used, without a selector for each place. Set the variables on `:root`:

```css
:root {
    --color-success: #009933;
    --color-error: #e60000;
}
```

### The colours of the theme

Each theme sets these. The settings **Colors & Themes** set some of them too.

| Variable | What it colours |
|---|---|
| `--color-base-100` | The background of the page |
| `--color-base-200` | Panels and boxes |
| `--color-base-300` | Borders and lines |
| `--color-base-content` | The text |
| `--color-primary`, `--color-primary-content` | Buttons, and the text on them |
| `--color-info`, `--color-info-content` | The accent colour, and text on it |
| `--color-success` | Things that work or are up |
| `--color-warning` | Things that need attention |
| `--color-error` | Things that failed or are down |

### The bar at the top and the menu

| Variable | What it colours |
|---|---|
| `--org-header-bg`, `--org-header-text` | The bar at the top, and its text |
| `--org-sidebar-bg`, `--org-sidebar-text` | The menu on the left, and its text |
| `--org-sidebar-active` | The tab that is open in the menu |

### The states of the calendar

| Variable | State | PHP class |
|---|---|---|
| `--org-event-downloaded` | Downloaded | `text-success` |
| `--org-event-unreleased` | Not released yet | `text-info` |
| `--org-event-missing` | Missing | `text-danger` |
| `--org-event-premiere` | Premiere | `text-primary` |
| `--org-event-unmonitored` | Not monitored | `text-dark` |
| `--org-event-upcoming` | An iCal event that is ahead | `text-info` |
| `--org-event-over` | An iCal event that is over | `text-dark` |

## Classes

### Homepage

| Class | What it is |
|---|---|
| `.org-item` | Each homepage item |
| `.org-item-<kind>` | One kind of item, for example `.org-item-healthchecks`, `.org-item-calendar`, `.org-item-monitorr`, `.org-item-uptimekuma` or `.org-item-customhtml`. The kinds are listed below |
| `.org-item-title` | The title of an item, with the line after it |
| `.org-count` | The number in the title, for example the number of health checks |
| `.org-cards` | A list of small cards: health checks, Monitorr, Uptime Kuma |
| `.org-card` | One card. Its left edge shows the state |
| `.org-card-row` | The name, the picture and the icon of a card |
| `.org-card-icon` | The icon of the state |
| `.org-card-tag` | A tag of a health check |
| `.org-card-more` | The times of a health check, which a click shows |
| `.org-tone-success`, `.org-tone-warning`, `.org-tone-error`, `.org-tone-info`, `.org-tone-primary` | The state of a card. Each sets the variable `--org-state`, which colours the edge and the icon |

Each item also has the attribute `data-item` with the key of the item, for example `data-item="homepageOrdercustomhtml01"`. Use it to aim at one of several items of the same kind.

The kinds of items: `adguard`, `calendar`, `customhtml`, `deluge`, `donate`, `downloader`, `embylivetv`, `embyrecent`, `embystreams`, `healthchecks`, `jackett`, `jdownloader`, `jellyfinrecent`, `jellyfinstreams`, `jellystat`, `monitorr`, `netdata`, `nzbget`, `octoprint`, `ombi`, `overseerr`, `pihole`, `plexplaylists`, `plexrecent`, `plexstreams`, `prompage`, `prowlarr`, `qbittorrent`, `radarrqueue`, `rtorrent`, `sabnzbd`, `sonarrqueue`, `speedtest`, `tautulli`, `transmission`, `unifi`, `uptimekuma`, `utorrent` and `weather`.

### Calendar

| Class | What it is |
|---|---|
| `.org-calendar` | The calendar |
| `.org-calendar-bar` | The bar with the month and the buttons |
| `.org-calendar-grid` | The table of the month, week or day view |
| `.org-today` | Today, in the table |
| `.org-calendar-event` | An event in the week and day views |
| `.org-calendar-time` | The time and the icon of an event |
| `.org-event-downloaded`, `.org-event-unreleased`, `.org-event-missing`, `.org-event-premiere`, `.org-event-unmonitored`, `.org-event-upcoming`, `.org-event-over` | The state of an event. Each takes its colour from the variable of the same name |

### Settings

| Class | What it is |
|---|---|
| `.org-panel` | A panel of the settings |
| `.org-panel-heading` | The title bar of a panel |
| `.org-panel-body` | The content of a panel |

### Login page and lock screen

| Class | What it is |
|---|---|
| `.org-login-register` | The login page under the bar at the top, with the wallpaper |
| `.org-login-sidebar` | The panel on the right that holds the form |
| `.org-login-box` | The login form in that panel |
| `.org-login-box-logo` | The logo or the name in the form. It shows on phones only |
| `.org-login-logo` | The picture of the logo |
| `.org-login-card` | The box of the minimal login page |
| `.org-lock-screen` | The lock screen, with the wallpaper |
| `.org-lock-box` | The box of the lock screen |

### Bar at the top and menu

| Class | What it is |
|---|---|
| `.org-header` | The bar at the top |
| `.org-sidebar` | The menu on the left |

## From PHP to this version

| PHP | This version |
|---|---|
| `#allHealthChecks`, `.healthCheckCards` | `.org-item-healthchecks`, `.org-item-healthchecks .org-cards` |
| `#allMonitorr`, `.monitorrCards` | `.org-item-monitorr`, `.org-item-monitorr .org-cards` |
| `#allUptimeKuma`, `.uptimeKumaCards` | `.org-item-uptimekuma`, `.org-item-uptimekuma .org-cards` |
| `.col-lg-3` and the other columns of a card | `.org-card` |
| `.left-health` | The left edge of `.org-card`. Change its colour with `--org-state` |
| `.bg-success`, `.text-success` on a card | `.org-tone-success { --org-state: ... }` |
| `.bg-danger`, `.text-danger` on a card | `.org-tone-error { --org-state: ... }` |
| `.healthCheckCards .label` | `.org-item-healthchecks .org-card-tag` |
| `#homepageOrder<item>` | `[data-item="homepageOrder<item>"]` or `.org-item-<kind>` |
| `.fc-toolbar` | `.org-calendar-bar` |
| `.fc-unthemed .fc-today` | `.org-calendar .org-today` |
| `.text-info`, `.text-primary` and the others in the calendar | The variables `--org-event-...` |
| `.panel`, `.panel-heading`, `.panel-body` in the settings | `.org-panel`, `.org-panel-heading`, `.org-panel-body` |
| `.login-register` | `.org-login-register` |
| `.login-box.login-sidebar`, `.login-sidebar` | `.org-login-sidebar` |
| `.login-sidebar .white-box` | `.org-login-sidebar .org-login-box` |
| `.white-box .visible-xs`, `#login-logo` | `.org-login-box-logo` |
| `.loginLogo` | `.org-login-logo` |
| `#lockScreen .white-box` | `.org-lock-screen .org-lock-box` |

Some parts of PHP have no counterpart here, so their rules have nothing to change: for example `.blockUI`, `.slideInRight` and the panels of the second factor in `#login-panels`.

### An example

This PHP rule puts the health checks in the middle, in cards of 210 pixels with 5 pixels of space on each side:

```css
.healthCheckCards {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
}
#allHealthChecks .col-lg-3 {
    width: 10%;
    min-width: 210px;
}
#allHealthChecks .col-md-4, #allHealthChecks .col-sm-6, #allHealthChecks .col-xs-12 {
    flex: 0 0 195px;
    padding-right: 5px;
    padding-left: 5px;
}
```

In PHP, a column holds the card. Here the card is the column, so the space between cards is a `gap`:

```css
.org-item-healthchecks .org-cards {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 10px;
}
.org-item-healthchecks .org-card {
    flex: 0 0 200px;
}
```

## When a rule does not apply

- The page loads custom CSS after its own style sheet, and outside its layers. So a rule of the same strength wins without `!important`.
- A style written on the element itself needs `!important`. The wallpaper of the login page and of the lock screen is such a style.
- Check the layout before you copy a PHP rule that moves things, such as `position: fixed`. The pages are laid out differently. For example, `.org-login-sidebar` is already placed over the full height of the page, so `left: 0; right: 0; margin: 0 auto` puts it in the middle.
