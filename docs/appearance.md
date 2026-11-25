# 🎨 Appearance

[← All documentation](README.md)

This page covers how Organizr looks: the two layouts, the themes, who picks which, the colors, and your own CSS and JavaScript. Read it when you want Organizr to look like home, or when you set the default look for everyone as Admin.

Most of it lives on **Settings › Customize › Appearance**. The theme marketplace is on **Settings › Customize › Marketplace**.

## 🪟 The two layouts

A layout is the frame around every page: the top bar and the menu. Organizr has two, and each person can use the one they like.

| | OG Organizr | Modern |
|---|---|---|
| What it is | The layout of PHP Organizr, as you know it | A fresh frame: a top bar, and a menu on the left that groups your tabs by category |
| Themes | 13 (`Organizr`, `Blue` and the 11 rebuilt marketplace themes) | 10, each with two or three color sets |
| Colors from **Style & Themes** | Yes | No, each modern theme brings its own |
| Moving scenes | No | On five themes |
| Settings pages | Tabs along the top, as in PHP | A side menu at the left |
| Starts on | Installs that you update or import from PHP | New installs |

<!-- SCREENSHOT: homepage-og.png | The homepage on the OG Organizr layout, Organizr theme, with a few tabs in the menu and a Plex item -->
<!-- SCREENSHOT: homepage-modern.png | The homepage on the modern layout, Organizr Dark theme, with the menu as a column of icons and one category open -->

A new install starts on the modern layout. An install that you update, or that you import from PHP Organizr, stays on the OG Organizr layout, so nobody sees a change until someone picks one.

### 🧭 The modern menu

On a screen 1024 pixels or wider, the modern menu works like the sidebar of PHP:

- It's a narrow column of icons beside the page. Each tab shows its picture or its first letter.
- A dot on each tab's picture shows whether the application of the tab answers.
- A category shows its picture. When the category is open, the pictures of its tabs show under it.
- Point at the menu, or move into it with the keyboard, and it opens over the page with all the names. It closes again when you leave it. On a touch screen it stays a column of icons.
- The button at the left of the top bar pins the menu open beside the page. Select it again to unpin it. This browser remembers your choice; another browser or device starts with the column of icons.
- The links that PHP adds at the end of the menu come last.

Below 1024 pixels of width, for example on a phone, the menu is a drawer that a button in the top bar opens. The dots still show whether each application answers.

### 🧱 What else changes on the modern layout

- **Settings**: a side menu at the left lists the same groups and pages as the tabs of OG Organizr. Below 1024 pixels, a **Settings menu** button opens the same list as a drawer. The top bar shows the name of the open page, for example "Settings · Tabs".
- **Homepage items**: cards in the colors of the theme, with a colored edge for each service where OG Organizr fills a box with the service's color.
- **Login and friends**: the login page, the pages to register, to recover a password and to use an invite, the lock screen, the set-up wizard and the page **Account** show a card in the theme's colors, over the wallpaper or the login picture.
- **Phones**: the top bar leaves out the name of the open tab, and a long user name ends in "…".
- **Everything else**: the other pages look as they do on the OG Organizr layout.
- **Motion**: all motion stops when your system asks for reduced motion.

## 🌈 The themes

### 👴 OG Organizr themes

Organizr has the two built-in themes of PHP Organizr, `Organizr` and `Blue`, plus the 11 themes of PHP's theme marketplace, rebuilt for these pages. Installing a marketplace theme on **Settings › Customize › Marketplace** downloads nothing: it only adds the theme to the list on **Appearance**.

Themes of other marketplaces aren't supported. Theme files written for PHP Organizr don't fit these pages.

### ✨ Modern themes

| Theme | Color sets | Look |
|---|---|---|
| `Organizr` | Dark, Light | The blue-gray colors of Organizr |
| `Plex` | Dark, Light | Charcoal with amber-gold |
| `Jellyfin` | Dark, Light | Deep navy, with a purple-to-blue gradient on the main buttons |
| `Emby` | Dark, Light | Near-black with green |
| `Silo` | Ember, Ice, Rose | Near-black with off-white buttons, one accent color and the font Plus Jakarta Sans. The color sets change the accent |
| `Nocturne` | Amethyst, Crimson, Moonlit | Gothic violet. A night sky with a moon, mist, a cathedral and bats. Headings in Cinzel |
| `Riot` | Acid, Hotwire, Blacklight | Punk neon. A brick wall with neon graffiti and signs. Headings in Sedgwick Ave Display |
| `Tempest` | Squall, Abyss, Gale | Stormy seas. Waves, rain, a lighthouse and now and then a soft flash of lightning. Headings in Pirata One |
| `Circuit` | Matrix, Synth, Amber | Cyberpunk. A circuit board with chips, traces and blinking lights. Headings in Orbitron |
| `Prism` | Daydream, Twilight, Candy | Sparkly rainbow. A rainbow, clouds and sparkles. All text in Fredoka. Daydream and Candy are light |

The Plex, Jellyfin, Emby and Silo themes borrow only the colors of those applications, never their logos.

### 🎬 The scenes

The last five themes (Nocturne, Riot, Tempest, Circuit and Prism) have a scene: a picture that moves slowly behind the page. It shows on the homepage, the settings and the page **Account**. Other pages cover it, and it stands still behind them.

The scene also stands still:

- when the Organizr browser tab is not in view,
- when a tab with another application's page fills the screen,
- behind the picture of the login page,
- when your computer or phone asks for reduced motion (for example **Reduce motion** in the system settings). The scene then stays as a still picture.

Text always sits on the theme's cards, never straight on the scene. The scene and the theme's font load only when you pick the theme.

## 👤 Who picks the look

The Admin sets the default look of the install. Anyone who is logged in can pick their own look with the **Look** picker. Guests, and people who haven't picked, see the default.

| | Default look (Admin) | Your own look (everyone) |
|---|---|---|
| Where | **Settings › Customize › Appearance**, section **Style & Themes** | The **Look** picker |
| Who sees it | Guests, and everyone who hasn't picked | Only you, on every device you log in from |
| Saved | When you select **Save** | At once, with each change |

### 👑 Set the default look (Admin)

<!-- SCREENSHOT: style-themes.png | Settings › Customize › Appearance, section Style & Themes, with Default layout set to Modern, the Modern theme and Modern color set with their color samples, the OG Organizr theme below, then Reset Colors and the color pickers under "Colors of the OG Organizr layout" -->

1. Open **Settings › Customize › Appearance**.
2. Open the section **Style & Themes**.
3. Choose **Default layout**. The theme of that layout shows below it, with a sample of its colors:
   - For OG Organizr: **Theme**.
   - For Modern: **Modern theme** and **Modern color set**. The OG **Theme** shows too, under the heading **OG Organizr**, for the people who pick the OG layout.
4. Select **Save** at the top of the panel. That one button saves the look and the colors of the section together.

Only the Admin can change the default layout, modern theme and color set. A Co-Admin sees them but can't change them. A Co-Admin can still change the OG **Theme** and the colors, and save them, as in PHP Organizr.

### 🪄 Pick your own look

<!-- SCREENSHOT: look-picker.png | The Look picker opened from the modern top bar: layout choice, the theme list and the color sets with their swatches, and the Use the default button -->

The **Look** picker is in three places:

- On the modern layout: the **Look** button in the top bar.
- On the OG Organizr layout: the **Look** entry in the user menu.
- On the page **Account**.

The picker has the layout and the theme, and on the modern layout the color set. Each theme and color set shows a small sample of its colors. On the OG Organizr layout, the sample of the install's own theme shows the button and accent colors that the Admin set in **Style & Themes**; the other samples show each theme's own colors.

Each change is saved at once. **Use the default** forgets your choices and brings back the Admin's default look.

Organizr remembers your last theme for each layout. Switch from OG Organizr to Modern and back, and OG Organizr still has the theme you picked there.

## 🎯 Colors of the OG Organizr layout

Below the themes in **Style & Themes**, the heading **Colors of the OG Organizr layout** stands over eight color pickers. They apply to the OG Organizr layout only; a modern theme sets its own colors.

| Field | Key | What it colors |
|---|---|---|
| Nav Bar Color | `headerColor` | The bar at the top |
| Nav Bar Text Color | `headerTextColor` | The text in that bar |
| Side Bar Color | `sidebarColor` | The menu |
| Side Bar Text Color | `sidebarTextColor` | The text in the menu |
| Accent Color | `accentColor` | Highlights |
| Accent Text Color | `accentTextColor` | Text on highlights |
| Button Color | `buttonColor` | Buttons |
| Button Text Color | `buttonTextColor` | Text on buttons |

- Each field takes a color, for example `#987654`, `rgb(20, 40, 60)` or `teal`. Organizr ignores a value that isn't a color.
- An empty field means the color of the theme.
- **Reset Colors** (the button **Reset**, above the pickers) empties all eight fields in the form. Select **Save** to keep that; until then nothing is stored.

Your [custom CSS](custom-css.md) comes after these colors, so it wins on both layouts.

## 🌄 The login page picture

The switch **Use Random Media Wallpaper From Media Server** (key `useRandomMediaImage`, off by default) is in the section **Login page** of **Appearance**. It shows a picture of something that was played as the background of the login page.

Organizr picks the picture at random from the pictures that the homepage keeps of what plays now on Plex, Emby or Jellyfin, and of the Tautulli statistics. A picture counts only when it is at least as wide as the "playing now" pictures: 1200 pixels times the setting **Image Cache Quality** (`cacheImageSize`). Until the homepage has kept such a picture, the login page shows the **Login Wallpaper**, or no picture (D-123).

## 🔖 Icons of the page

The setting `favIcon` of PHP Organizr holds HTML for the head of the page, as a favicon generator makes it. Put that HTML in the field **Fav Icon Code**, in the section **Notifications and favicon** of **Settings › Customize › Appearance**.

Organizr keeps only the icon links of that HTML:

- `<link>` elements with `rel` set to `icon`, `shortcut icon` or `apple-touch-icon`,
- whose address is on the same site or starts with `https://`,
- at most 10 of them.

All other HTML of the setting is dropped.

Icons that the import copied from PHP's `data/favicon` folder are served at their old address, `data/favicon/<name>`, so the links of an imported setting keep working. Organizr sends only GIF, PNG, JPEG, ICO and SVG files from that folder; other files, such as `site.webmanifest`, get "not found".

## 💅 Custom CSS

<!-- SCREENSHOT: custom-code-page.png | Settings › Customize › Appearance, section Custom code, signed in as Admin: the Custom CSS and Theme CSS fields, the note that says whether the scripts run (ORGANIZR_CUSTOM_JS) with its warning, and the Custom Javascript and Theme Javascript fields -->

The section **Custom code** of **Settings › Customize › Appearance** has two style sheets, **Custom CSS** and **Theme CSS**, for every page on both layouts.

- They load after the theme's style sheet, so their rules win.
- The server serves them as one file, `api/v2/appearance/custom.css` (Theme CSS first, then Custom CSS).
- The page loads styles only from its own server. An `@import` or a font from another site in your CSS does not load.
- Everybody can read that file, guests too, so keep secrets out of it.

Custom CSS written for PHP Organizr aims at names these pages don't have. [Custom CSS](custom-css.md) lists the names to use, and shows how to change a PHP rule.

## 📜 Custom JavaScript

The same section has two scripts, **Custom Javascript** and **Theme Javascript** (that's how the labels are spelled on the page). Only the Admin sees and changes them.

The scripts do **not** run until the person who runs the server turns them on:

| Variable | Meaning | Default |
| --- | --- | --- |
| `ORGANIZR_CUSTOM_JS` | `1` runs the custom scripts on every page | Off |

> [!WARNING]
> A script runs in the session of every person who opens Organizr, guests on the login page and locked accounts too. It can do all that person can do: read the pages, call the API as them, and change their account. Turn the scripts on only when you trust every person who can log in as Admin.

Why the extra switch? A script runs with the rights of everyone who opens the page, including the Admin. Because it's an environment variable, turning it on takes access to the server itself, not only an Admin login in the browser. Someone who only steals an Admin session can't make a script run for everybody.

How it works:

- With the variable set, the server serves both scripts as one file, `api/v2/appearance/custom.js` (Theme Javascript first), and the page loads it from there.
- Without the variable, or without a script, that file doesn't exist and the page loads no script.
- The **Custom code** section has a note that says whether the scripts run now.
- The scripts don't run in the set-up wizard.

> [!CAUTION]
> Every visitor can read the text of the scripts, guests too, even while the scripts are switched off (`GET /launch` hands them out). Never put a key, a token or a password in them.

Scripts written for PHP Organizr often use jQuery or the element ids of the old pages. They don't work here and must be rewritten.

## 📚 Details

### 🔧 Setting keys

| Key | Values | Default |
|---|---|---|
| `layout` | `og` or `modern` | `og`; the set-up wizard of a new install stores `modern` |
| `modernTheme` | `organizr`, `plex`, `jellyfin`, `emby`, `silo`, `nocturne`, `riot`, `tempest`, `circuit` or `prism` | `organizr` |
| `modernColorSet` | The id of one of the theme's color sets, in lower case, for example `dark`, `ember` or `daydream` | `dark` |
| `theme` | The OG theme: `Organizr`, `Blue` or an installed marketplace theme | `Organizr` |
| `style` | `light`, `dark` or `horizontal` | `dark` |

- Only the Admin can change `layout`, `modernTheme` and `modernColorSet`.
- A color set that the theme doesn't have gives way to the theme's first color set. For example `silo` with `dark` shows Silo Ember.
- `style` is PHP's setting. Organizr stores it, but it has no effect (each theme brings its own light or dark colors), so the page doesn't show a field for it (D-92).

### 🔌 The API

- `GET /launch` has the field `look` with the layout, theme, color set and source of the look that applies.
- `PUT /api/v2/user/look` stores the choice of the person who is logged in, with the fields `layout`, `theme` and `colorSet`.
  - A body that names only `layout` switches the layout and keeps that layout's last theme and colors.
  - A `layout` of `null` forgets the whole choice.
- A script with an API key has no look of its own.

D-125 in [Deviations](deviations.md) has the details; D-92 and D-93 cover the themes, colors, CSS, scripts and icons.
