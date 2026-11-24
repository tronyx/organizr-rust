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

Each theme sets these. On the OG Organizr layout, the settings **Colors & Themes** set some of them too. A modern theme sets its own colours, so those settings do not change them. Your custom CSS comes after the theme, so your variables win on both layouts.

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

The modern layout sets these variables from the colours of its theme. A rule in your custom CSS for one of them still wins on both layouts.

### The colours of the services

The colour of each service is a variable too, for example `--org-brand-plex`, `--org-brand-emby` and `--org-brand-sonarr`. The modern layout uses them for the coloured edge of the tiles of **Homepage Order**. Change one to change that edge.

## The layout and the theme

Custom CSS applies on both layouts. The element `<html>` has attributes that tell which look is on, so a rule can aim at one of them:

| Attribute | Value |
|---|---|
| `data-layout` | `og` for the OG Organizr layout, `modern` for the modern layout |
| `data-theme` | The theme. On the modern layout it is `m-` followed by the theme and its colour set, for example `m-organizr-dark`, `m-plex-light` or `m-silo-ember` |
| `data-colour-set` | `light` when the colours of the modern theme are light, else `dark`. For example Silo is always `dark`, and Prism Daydream is `light`. The OG Organizr layout does not have it |

To hide every scene, add `.m-scene { display: none; }`. The page then shows the plain colour of the theme. The scene still loads, so the theme still costs the same to open.

A rule for the modern layout only starts with `[data-layout='modern']`:

```css
[data-layout='modern'] {
    --org-header-bg: #1f4e79;
}
```

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

The kinds of items: `adguard`, `bookmarks`, `calendar`, `customhtml`, `deluge`, `donate`, `downloader`, `embylivetv`, `embyrecent`, `embystreams`, `healthchecks`, `jackett`, `jdownloader`, `jellyfinrecent`, `jellyfinstreams`, `jellystat`, `monitorr`, `netdata`, `nzbget`, `octoprint`, `ombi`, `overseerr`, `pihole`, `plexplaylists`, `plexrecent`, `plexstreams`, `prompage`, `prowlarr`, `qbittorrent`, `radarrqueue`, `rtorrent`, `sabnzbd`, `sonarrqueue`, `speedtest`, `tautulli`, `transmission`, `unifi`, `uptimekuma`, `utorrent` and `weather`.

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

The other classes of the settings, such as `.org-table` and `.org-form-control`, are the same on both layouts. So a rule for them applies on both layouts, unless it starts with `[data-layout='og']` or `[data-layout='modern']`.

The modern layout also has these classes for the settings. They exist on the modern layout only:

| Class | What it is |
|---|---|
| `.m-settings` | The settings page, beside the side menu |
| `.m-settings-side` | The column that holds the side menu |
| `.m-settings-nav` | The side menu |
| `.m-settings-group` | One group of pages in the side menu, with its title |
| `.m-settings-button` | The button **Settings menu**, below 1024 pixels. It opens the drawer |
| `.m-settings-button-text`, `.m-settings-button-label`, `.m-settings-button-place` | The text in that button: the box, the label and the name of the open page |
| `.m-settings-sheet` | The drawer that holds the menu |
| `.m-settings-sheet-head` | The title of the drawer |
| `.m-page-heading` | The heading of the open page, with the name of its group below it |

Some homepage items have these classes on both layouts. On the modern layout they get their colours from the theme:

| Class | What it is |
|---|---|
| `.org-adguard-card` | A card of numbers in the AdGuard Home item. The item sets its colour itself, so change it in custom CSS with `background-color`, `color` or `box-shadow` |
| `.org-speedtest-icon` | The icon of a card in the Speedtest item. The item sets its colour itself, so change it in custom CSS with `color` or `background-color` |
| `.org-tile` | A dark tile in the Monitorr, UniFi, OctoPrint and Speedtest items |
| `.org-fill` | A box filled with the colour of its state, with `.org-tone-success`, `.org-tone-warning`, `.org-tone-error`, `.org-tone-info`, `.org-tone-primary` or `.org-tone-neutral` |
| `.org-calendar-other` | A day of the calendar outside the month that it shows |
| `.org-netdata-track` | The empty track of a gauge in the Netdata item |

### Login page and lock screen

| Class | What it is |
|---|---|
| `.org-login-register` | The login page under the bar at the top, with the wallpaper |
| `.org-login-sidebar` | The panel on the right that holds the form |
| `.org-login-box` | The login form in that panel |
| `.org-login-box-logo` | The logo or the name in the form. It shows on phones only |
| `.org-login-logo` | The picture of the logo |
| `.org-login-card` | The box of the minimal login page |
| `.org-login-form` | The form on the login page and on the pages to register, to recover a password and to use an invite |
| `.org-login-title` | The heading of that form |
| `.org-lock-screen` | The lock screen, with the wallpaper |
| `.org-lock-box` | The box of the lock screen |
| `.org-lock-name` | The name of the person on the lock screen |

### Bar at the top and menu

| Class | What it is |
|---|---|
| `.org-header` | The bar at the top |
| `.org-header-link` | A link or button in the bar at the top |
| `.org-logo` | The logo or the name in the bar at the top |
| `.org-sidebar` | The menu on the left |
| `.org-sidebar-head` | The title of the menu |
| `.org-menu` | The list of tabs in the menu |

The modern layout uses these same classes, so a rule for them applies on both layouts. The modern layout also has these classes. They exist on the modern layout only:

| Class | What it is |
|---|---|
| `.m-shown` | The name of the open tab in the bar at the top |
| `.m-look` | The box of the button **Look** in the bar at the top |
| `.m-look-panel` | The look picker that the button **Look** opens |
| `.m-user` | The button of the user in the bar at the top |
| `.m-user-name` | The name in that button. On a phone a long name ends in "…" |
| `.m-scene` | The moving scene behind the page, on the themes Nocturne, Riot, Tempest, Circuit and Prism |
| `.m-scene-nocturne`, `.m-scene-riot`, `.m-scene-tempest`, `.m-scene-circuit`, `.m-scene-prism` | The scene of one theme |
| `.m-scene-paused` | On `.m-scene` while the scene stands still |
| `.m-with-menu` | The page area, when the menu is beside it. Below 1024 pixels the menu is a drawer, and the class is absent |
| `.org-menu-extras` | The links at the end of the menu |

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
