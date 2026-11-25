# 💅 Custom CSS

[← All documentation](README.md)

This page lists the CSS names you can safely aim at, and shows how to turn a PHP Organizr rule into one that works here. Read it when you want to restyle Organizr, or when your old custom CSS stopped doing anything.

## 🚀 Quick example

Want greener "up" colors and rounder health check cards? This is all it takes:

```css
:root {
    --color-success: #2ecc71;
}
.org-item-healthchecks .org-card {
    border-radius: 12px;
}
```

1. Open **Settings › Customize › Appearance**.
2. Open the section **Custom code**, and paste the rule into the field **Custom CSS**.
3. Select **Save**.
4. Reload the page. Everything that means "works" or "up" is now your green, and the health check cards have round corners.

[Appearance](appearance.md#-custom-css) tells you how the page loads your CSS (after the theme, so your rules win).

## 🤔 Why PHP rules don't work here

This version of Organizr builds its pages differently from PHP Organizr. Custom CSS written for PHP aims at PHP's ids and classes, such as `#allHealthChecks` or `.white-box`, and those names aren't on these pages. Use the names on this page instead; [From PHP to this version](#-from-php-to-this-version) has a lookup table.

## 🤝 The names you can rely on

The CSS variables and classes on this page are stable: a release doesn't change them without a note in the release notes. The project's tests fail if the interface stops using one of them, or if custom CSS stops reaching them.

The pages have other classes too, for example Tailwind classes such as `flex` or `px-4`. Don't aim at those. They can change in any release.

## 🎨 Colors: CSS variables

Most custom CSS only changes colors. A CSS variable changes a color everywhere it's used, without a selector for each place. Set the variables on `:root`:

```css
:root {
    --color-success: #009933;
    --color-error: #e60000;
}
```

### 🌈 The theme colors

Each theme sets these. On the OG Organizr layout, the colors in **Style & Themes** set some of them too. A modern theme sets its own colors, so those settings don't change them. Your custom CSS comes after the theme, so your variables win on both layouts.

| Variable | What it colors |
|---|---|
| `--color-base-100` | The background of the page |
| `--color-base-200` | Panels and boxes |
| `--color-base-300` | Borders and lines |
| `--color-base-content` | The text |
| `--color-primary`, `--color-primary-content` | Buttons, and the text on them |
| `--color-info`, `--color-info-content` | The accent color, and text on it |
| `--color-success` | Things that work or are up |
| `--color-warning` | Things that need attention |
| `--color-error` | Things that failed or are down |

### 🧭 The top bar and the menu

| Variable | What it colors |
|---|---|
| `--org-header-bg`, `--org-header-text` | The bar at the top, and its text |
| `--org-sidebar-bg`, `--org-sidebar-text` | The menu on the left, and its text |
| `--org-sidebar-active` | The tab that is open in the menu |
| `--org-swatch-edge` | The border of each color sample (`.org-swatch`) in the look picker. By default 55 % of the page's text color mixed into its background, about 3:1 |

### 📅 The calendar states

| Variable | State | PHP class |
|---|---|---|
| `--org-event-downloaded` | Downloaded | `text-success` |
| `--org-event-unreleased` | Not released yet | `text-info` |
| `--org-event-missing` | Missing | `text-danger` |
| `--org-event-premiere` | Premiere | `text-primary` |
| `--org-event-unmonitored` | Not monitored | `text-dark` |
| `--org-event-upcoming` | An iCal event that is ahead | `text-info` |
| `--org-event-over` | An iCal event that is over | `text-dark` |

The modern layout sets these from the colors of its theme. A rule in your custom CSS for one of them still wins on both layouts.

### 🔖 The service colors

Each service has a color variable too, for example `--org-brand-plex`, `--org-brand-emby` and `--org-brand-sonarr`. The modern layout uses them for the colored edge of the tiles on **Homepage Order**. Change one to change that edge.

### 🚦 The state of a card

The tone classes (`.org-tone-success` and its siblings, listed under [Homepage items](#-homepage-items)) each set the variable `--org-state`, which colors the edge and the icon of a card. Set `--org-state` inside one of them to recolor that state.

## 🎯 Aim at one layout or theme

Custom CSS applies on both layouts. The `<html>` element has attributes that tell which look is on, so a rule can aim at one of them:

| Attribute | Value |
|---|---|
| `data-layout` | `og` for the OG Organizr layout, `modern` for the modern layout |
| `data-theme` | The theme. On the modern layout it's `m-` followed by the theme and its color set, for example `m-organizr-dark`, `m-plex-light` or `m-silo-ember` |
| `data-color-set` | `light` when the modern theme's colors are light, else `dark`. For example Silo is always `dark`, and Prism Daydream is `light`. The OG Organizr layout doesn't have it |

A rule for the modern layout only starts with `[data-layout='modern']`:

```css
[data-layout='modern'] {
    --org-header-bg: #1f4e79;
}
```

To hide every scene, add `.m-scene { display: none; }`. The page then shows the theme's plain color. The scene still loads, so the theme still costs the same to open.

## 🧩 Classes, by part of the page

The **Layout** column says where a class exists: **Both** for both layouts, **Modern** for the modern layout only.

### 🧭 Top bar and menu

| Class | Layout | What it is |
|---|---|---|
| `.org-header` | Both | The bar at the top |
| `.org-header-link` | Both | A link or button in the bar at the top |
| `.org-logo` | Both | The logo or the name in the bar at the top |
| `.org-sidebar` | Both | The menu on the left |
| `.org-sidebar-head` | Both | The title of the menu |
| `.org-menu` | Both | The list of tabs in the menu |
| `.org-menu-extras` | Modern | The links at the end of the menu |
| `.org-swatch` | Both | A color sample of a theme in the look picker |
| `.org-swatch-dot` | Both | One color of that sample: the text, primary and accent colors of the theme |
| `.m-shown` | Modern | The name of the open tab in the bar at the top |
| `.m-look` | Modern | The box of the **Look** button in the bar at the top |
| `.m-look-panel` | Modern | The look picker that the **Look** button opens |
| `.m-user` | Modern | The user button in the bar at the top |
| `.m-user-name` | Modern | The name in that button. On a phone a long name ends in "…" |
| `#m-menu` | Modern | The menu at the left on a wide screen (an `aside` with `.org-sidebar`) |
| `.m-sidebar-icons` | Modern | On `.org-sidebar` while the menu is a column of icons. The column is `4rem` wide |
| `.m-sidebar-over` | Modern | On `.org-sidebar` while the menu is open over the page: under the pointer or with the keyboard in it, and not pinned |
| `.m-dot` | Modern | The dot of a tab in the column of icons, on the corner of its picture. It shows whether the tab's application answers |
| `.m-with-menu` | Modern | The page area, when the menu is beside it. Below 1024 pixels the menu is a drawer, and the class is absent |
| `.m-with-icons` | Modern | On the page area, with `.m-with-menu`, while the menu is not pinned. The page then stands beside the column of icons, also while the menu is open over it |

### 🎬 Scenes

| Class | Layout | What it is |
|---|---|---|
| `.m-scene` | Modern | The moving scene behind the page, on the themes Nocturne, Riot, Tempest, Circuit and Prism |
| `.m-scene-nocturne`, `.m-scene-riot`, `.m-scene-tempest`, `.m-scene-circuit`, `.m-scene-prism` | Modern | The scene of one theme |
| `.m-scene-paused` | Modern | On `.m-scene` while the scene stands still |

### 🏠 Homepage items

| Class | Layout | What it is |
|---|---|---|
| `.org-item` | Both | Each homepage item |
| `.org-item-<kind>` | Both | One kind of item, for example `.org-item-healthchecks`, `.org-item-calendar`, `.org-item-monitorr`, `.org-item-uptimekuma` or `.org-item-customhtml`. The kinds are listed below the table |
| `.org-item-title` | Both | The title of an item, with the line after it |
| `.org-count` | Both | The number in the title, for example the number of health checks |
| `.org-cards` | Both | A list of small cards: health checks, Monitorr, Uptime Kuma |
| `.org-card` | Both | One card. Its left edge shows the state |
| `.org-card-row` | Both | The name, the picture and the icon of a card |
| `.org-card-icon` | Both | The icon of the state |
| `.org-card-tag` | Both | A tag of a health check |
| `.org-card-more` | Both | The times of a health check, which a click shows |
| `.org-tone-success`, `.org-tone-warning`, `.org-tone-error`, `.org-tone-info`, `.org-tone-primary` | Both | The state of a card. Each sets the variable `--org-state`, which colors the edge and the icon |
| `.org-fill` | Both | A box filled with the color of its state, with `.org-tone-success`, `.org-tone-warning`, `.org-tone-error`, `.org-tone-info`, `.org-tone-primary` or `.org-tone-neutral` |
| `.org-tile` | Both | A dark tile in the Monitorr, UniFi, OctoPrint and Speedtest items |
| `.org-adguard-card` | Both | A card of numbers in the AdGuard Home item. The item sets its color itself, so change it with `background-color`, `color` or `box-shadow` |
| `.org-speedtest-icon` | Both | The icon of a card in the Speedtest item. The item sets its color itself, so change it with `color` or `background-color` |
| `.org-netdata-track` | Both | The empty track of a gauge in the Netdata item |

On the modern layout, `.org-tile`, `.org-fill`, `.org-adguard-card`, `.org-speedtest-icon` and `.org-netdata-track` get their colors from the theme.

Each item also has the attribute `data-item` with the item's key, for example `data-item="homepageOrdercustomhtml01"`. Use it to aim at one of several items of the same kind.

The kinds of items: `adguard`, `bookmarks`, `calendar`, `customhtml`, `deluge`, `donate`, `downloader`, `embylivetv`, `embyrecent`, `embystreams`, `healthchecks`, `jackett`, `jdownloader`, `jellyfinrecent`, `jellyfinstreams`, `jellystat`, `monitorr`, `netdata`, `netronome`, `nzbget`, `octoprint`, `ombi`, `overseerr`, `pihole`, `plexplaylists`, `plexrecent`, `plexstreams`, `prompage`, `prowlarr`, `qbittorrent`, `radarrqueue`, `rtorrent`, `sabnzbd`, `sonarrqueue`, `speedtest`, `tautulli`, `transmission`, `unifi`, `uptimekuma`, `utorrent` and `weather`.

### 📅 Calendar

| Class | Layout | What it is |
|---|---|---|
| `.org-calendar` | Both | The calendar |
| `.org-calendar-bar` | Both | The bar with the month and the buttons |
| `.org-calendar-grid` | Both | The table of the month, week or day view |
| `.org-calendar-other` | Both | A day outside the month that the calendar shows |
| `.org-today` | Both | Today, in the table |
| `.org-calendar-event` | Both | An event in the week and day views |
| `.org-calendar-time` | Both | The time and the icon of an event |
| `.org-event-downloaded`, `.org-event-unreleased`, `.org-event-missing`, `.org-event-premiere`, `.org-event-unmonitored`, `.org-event-upcoming`, `.org-event-over` | Both | The state of an event. Each takes its color from the variable of the same name |

### 🔑 Login page and lock screen

| Class | Layout | What it is |
|---|---|---|
| `.org-login-register` | Both | The login page under the bar at the top, with the wallpaper |
| `.org-login-sidebar` | Both | The panel on the right that holds the form |
| `.org-login-box` | Both | The login form in that panel |
| `.org-login-box-logo` | Both | The logo or the name in the form. It shows on phones only |
| `.org-login-logo` | Both | The picture of the logo |
| `.org-login-card` | Both | The box of the minimal login page |
| `.org-login-form` | Both | The form on the login page and on the pages to register, to recover a password and to use an invite |
| `.org-login-title` | Both | The heading of that form |
| `.org-lock-screen` | Both | The lock screen, with the wallpaper |
| `.org-lock-box` | Both | The box of the lock screen |
| `.org-lock-name` | Both | The name of the person on the lock screen |

### 🔧 Settings

| Class | Layout | What it is |
|---|---|---|
| `.org-panel` | Both | A panel of the settings |
| `.org-panel-heading` | Both | The title bar of a panel |
| `.org-panel-body` | Both | The content of a panel |
| `.org-click-row` | Both | A table row that opens its item with a click |
| `.m-settings` | Modern | The settings page, beside the side menu |
| `.m-settings-side` | Modern | The column that holds the side menu |
| `.m-settings-nav` | Modern | The side menu |
| `.m-settings-group` | Modern | One group of pages in the side menu, with its title |
| `.m-settings-button` | Modern | The **Settings menu** button, below 1024 pixels. It opens the drawer |
| `.m-settings-button-text`, `.m-settings-button-label`, `.m-settings-button-place` | Modern | The text in that button: the box, the label and the name of the open page |
| `.m-settings-sheet` | Modern | The drawer that holds the menu |
| `.m-settings-sheet-head` | Modern | The title of the drawer |
| `.m-page-heading` | Modern | The heading of the open page, with the name of its group below it |

The other settings classes, such as `.org-table` and `.org-form-control`, are the same on both layouts. So a rule for them applies on both layouts, unless it starts with `[data-layout='og']` or `[data-layout='modern']`.

#### 👥 The Manage Users table

| Class | Layout | What it is |
|---|---|---|
| `.org-users-table` | Both | The table of the users |
| `.org-users-scroll` | Both | The box around the table, which scrolls sideways when the table is wider |
| `.org-users-avatar`, `.org-users-email`, `.org-users-date` | Both | The cells of the columns Avatar, Email and Date Registered. On the modern layout, in a narrow card, the columns Avatar and Date Registered are left out, a long address is cut short, and the date shows under the name |
| `.org-users-date-line` | Both | The date under the name, in a narrow card on the modern layout |
| `.org-users-actions` | Both | The Edit and Delete buttons of a row |

## 🔄 From PHP to this version

| PHP | This version |
|---|---|
| `#allHealthChecks`, `.healthCheckCards` | `.org-item-healthchecks`, `.org-item-healthchecks .org-cards` |
| `#allMonitorr`, `.monitorrCards` | `.org-item-monitorr`, `.org-item-monitorr .org-cards` |
| `#allUptimeKuma`, `.uptimeKumaCards` | `.org-item-uptimekuma`, `.org-item-uptimekuma .org-cards` |
| `.col-lg-3` and the other columns of a card | `.org-card` |
| `.left-health` | The left edge of `.org-card`. Change its color with `--org-state` |
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

Some parts of PHP have no counterpart here, so their rules have nothing to change: for example `.blockUI`, `.slideInRight` and the second-factor panels in `#login-panels`.

### 🔨 A worked conversion

This PHP rule centers the health checks, in cards of 210 pixels with 5 pixels of space on each side:

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

## 🩺 When a rule does not apply

- The page loads custom CSS after its own style sheet, and outside its layers. So a rule of the same strength wins without `!important`.
- A style written on the element itself needs `!important`. The wallpaper of the login page and of the lock screen is such a style.
- Check the layout before you copy a PHP rule that moves things, such as `position: fixed`. The pages are laid out differently. For example, `.org-login-sidebar` is already placed over the full height of the page, so `left: 0; right: 0; margin: 0 auto` puts it in the middle.
- An `@import` or a font from another site doesn't load: the page takes styles only from its own server.
