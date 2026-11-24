# Appearance

The settings page **Appearance** changes how Organizr looks. The settings page **Themes** holds the theme marketplace.

## Layouts and themes

A layout is the frame around every page: the top bar and the menu. Organizr has two layouts:

- **OG Organizr** is the layout of PHP Organizr. It has the 13 themes described under [Themes](#themes).
- **Modern** has a top bar and a menu on the left that groups the tabs by category. Below 1024 pixels of width, for example on a phone, the menu is a drawer that a button in the top bar opens. A dot at each tab shows whether the application of the tab answers. The links that PHP adds at the end of the menu come last. Modern has these themes:

| Theme | Colour sets | Look |
|---|---|---|
| `Organizr` | Dark, Light | The blue-grey colours of Organizr |
| `Plex` | Dark, Light | Charcoal with amber-gold |
| `Jellyfin` | Dark, Light | Deep navy, with a purple-to-blue gradient on the main buttons |
| `Emby` | Dark, Light | Near-black with green |
| `Silo` | Ember, Ice, Rose | Near-black with off-white buttons, one accent colour and the font Plus Jakarta Sans. The colour sets change the accent |
| `Nocturne` | Amethyst, Crimson, Moonlit | Gothic violet. A night sky with a moon, mist, a cathedral and bats. Headings in Cinzel |
| `Riot` | Acid, Hotwire, Blacklight | Punk neon. A brick wall with neon graffiti and signs. Headings in Sedgwick Ave Display |
| `Tempest` | Squall, Abyss, Gale | Stormy seas. Waves, rain, a lighthouse and now and then a soft flash of lightning. Headings in Pirata One |
| `Circuit` | Matrix, Synth, Amber | Cyberpunk. A circuit board with chips, traces and blinking lights. Headings in Orbitron |
| `Prism` | Daydream, Twilight, Candy | Sparkly rainbow. A rainbow, clouds and sparkles. All text in Fredoka. Daydream and Candy are light |

The themes Plex, Jellyfin, Emby and Silo take only the colours of these applications, not their logos.

### The scenes

The last five themes have a scene: a picture that moves slowly behind the page. The scene shows on the homepage, the settings and the page **Account**. The other pages cover it, and it stands still behind them. It also stands still:

- when the tab of Organizr is not in view,
- when a tab with the page of another application fills the screen,
- behind the picture of the login page,
- when your computer or phone asks for reduced motion (for example **Reduce motion** in the settings of the system). The scene then stays as a still picture.

Every text stays on the cards of the theme, never on the scene. The scene, and the font of its theme, load only when you pick the theme.

The settings on the modern layout have a side menu at the left. It lists the same groups and pages as the tabs of OG Organizr. Below 1024 pixels of width, a **Settings menu** button opens the same list as a drawer. The top bar shows the name of the open page, for example "Settings · Tabs". The homepage items have the modern look too: cards in the colours of the theme, and a coloured edge for each service where OG Organizr fills a box with the colour of the service. The login page, the pages to register, to recover a password and to use an invite, the lock screen, the set-up wizard and the page **Account** have the modern look too: a card in the colours of the theme, over the wallpaper or the picture of the login page. On a phone the bar at the top leaves out the name of the open tab, and a long user name ends in "…". The other pages inside the modern layout look as they look in the OG Organizr layout.

An install that you update stays on the OG Organizr layout. Nobody sees a change until someone chooses one.

### Who chooses

The Admin sets the default look of the install. A person who is logged in can choose their own look. A guest, and a person who has not chosen, sees the default.

To set the default look, as Admin:

1. Open **Settings**, then **Appearance**.
2. Open the group **Layout**.
3. Choose **Default layout**, **Modern theme** and **Modern colour set**.
4. Select **Save the layout**.

The settings are `layout` (`og` or `modern`, `og` by default), `modernTheme` (`organizr`, `plex`, `jellyfin`, `emby`, `silo`, `nocturne`, `riot`, `tempest`, `circuit` or `prism`; `organizr` by default) and `modernColourSet` (the id of a colour set of that theme in lower case, for example `dark`, `ember` or `daydream`; `dark` by default). Only the Admin can change them. A Co-Admin sees them but cannot save. A colour set that the theme does not have gives way to the first colour set of the theme, for example `silo` with `dark` shows Silo Ember.

To choose your own look, use the **Look** picker. You find it in three places:

- In the modern layout, the button **Look** in the top bar.
- In the OG Organizr layout, the entry **Look** in the menu of the user.
- On the page **Account**.

The picker has the layout and the theme, and on the modern layout the colours. Each change is saved at once. The button **Use the default** forgets your choices and returns you to the default look of the Admin.

### What each layout remembers

Organizr remembers your last theme for each layout. If you switch from the OG Organizr layout to the modern one and back, the OG Organizr layout keeps the theme that you chose there.

### The API

`GET /launch` has the field `look` with the layout, theme, colour set and source of the look that applies. `PUT /api/v2/user/look` stores the choice of the person who is logged in, with the fields `layout`, `theme` and `colourSet`. A body that names only `layout` switches the layout and keeps the last theme and colours of that layout. A `layout` of `null` forgets the whole choice. A script with an API key has no look of its own. D-125 in [Deviations](deviations.md) has the details.

### Custom CSS and colours

[Custom CSS](custom-css.md) applies on both layouts. The settings **Colors & Themes** apply on the OG Organizr layout only. A modern theme sets its own colours. Motion in the modern layout stops when the system asks for reduced motion.

## Themes

Organizr has the two themes of PHP Organizr, `Organizr` and `Blue`, and the 11 themes of the theme marketplace of PHP Organizr. Organizr rebuilt these themes for its own pages. An install of a marketplace theme downloads nothing: it only adds the theme to the list on the page **Appearance**.

Themes of other marketplaces are not supported. Theme files that were written for PHP Organizr do not fit the pages of this application.

## Colours

These fields apply on the OG Organizr layout only. A modern theme sets its own colours.

Each colour field takes a colour, for example `#987654`, `rgb(20, 40, 60)` or `teal`. An empty field means the colour of the theme. Organizr ignores a value that is not a colour.

## The picture of the login page

The switch **Use Random Media Wallpaper From Media Server**, on the page **Appearance**, group **Login Page**, shows a picture of something that was played as the background of the login page. Organizr chooses the picture at random from the pictures that the homepage keeps of what plays now on Plex, Emby or Jellyfin and of the statistics of Tautulli. A picture counts only when it is at least as wide as the pictures of what plays: 1200 pixels times the setting **Image Cache Quality** (`cacheImageSize`). Until the homepage has kept such a picture, the login page shows the **Login Wallpaper**, or no picture. Setting key: `useRandomMediaImage`. Off by default (D-123).

## Custom CSS

**Custom CSS** and **Theme CSS** are style sheets for every page. They come after the style sheet of the theme, so their rules win. The server serves them as one file, `api/v2/appearance/custom.css`.

The page loads styles only from its own server. An `@import` or a font from another site in the custom CSS does not load.

Custom CSS written for PHP Organizr aims at names that these pages do not have. [Custom CSS](custom-css.md) lists the names to use, and how to change a PHP rule.

## Custom JavaScript

**Custom JavaScript** and **Theme JavaScript** are scripts that run on every page. Only the Admin sees and changes them.

> **Warning:** a script runs in the session of every person who opens Organizr, also guests on the login page. It can do all that the person can do: read the pages, call the API as that person, and change the account. Turn the scripts on only when you trust every person who can log in as Admin.

The scripts do not run unless the operator of the server turns them on:

| Variable | Meaning | Default |
| --- | --- | --- |
| `ORGANIZR_CUSTOM_JS` | `1` runs the custom scripts on every page | Off |

With the variable set, the server serves the scripts as one file, `api/v2/appearance/custom.js`, and the page loads it from there. Without the variable, or without a script, the file does not exist and the page loads no script. The page **Appearance** says whether the scripts run now.

Scripts that were written for PHP Organizr often use jQuery or the element ids of the old pages. They do not work here and must be rewritten.

## Icons of the page

The setting `favIcon` of PHP Organizr holds HTML for the head of the page, as a favicon generator makes it. Organizr keeps only the icon links of that HTML: `<link>` elements with `rel` set to `icon`, `shortcut icon` or `apple-touch-icon`, and an address on the same site or one that starts with `https://`. All other HTML of the setting is dropped.

Put the HTML into the field **Icon code** on the page **Appearance**. Organizr does not serve the folder `data/favicon` of PHP Organizr yet, so an icon link to that folder finds no file. Use an address of another site that starts with `https://` for now.
