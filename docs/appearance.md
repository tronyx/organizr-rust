# Appearance

The settings page **Appearance** changes how Organizr looks. The settings page **Themes** holds the theme marketplace.

## Layouts and themes

A layout is the frame around every page: the top bar and the menu. Organizr has two layouts:

- **OG Organizr** is the layout of PHP Organizr. It has the 13 themes described under [Themes](#themes).
- **Modern** has a top bar and a menu on the left that groups the tabs by category. Below 1024 pixels of width, for example on a phone, the menu is a drawer that a button in the top bar opens. A dot at each tab shows whether the application of the tab answers. The links that PHP adds at the end of the menu come last. Modern has one theme for now, `Organizr`, in dark colours. More themes, and light colours, come with the next versions of the modern layout.

The settings on the modern layout have a side menu at the left. It lists the same groups and pages as the tabs of OG Organizr. Below 1024 pixels of width, a **Settings menu** button opens the same list as a drawer. The top bar shows the name of the open page, for example "Settings · Tabs". The other pages inside the modern layout look as they look in the OG Organizr layout.

An install that you update stays on the OG Organizr layout. Nobody sees a change until someone chooses one.

### Who chooses

The Admin sets the default look of the install. A person who is logged in can choose their own look. A guest, and a person who has not chosen, sees the default.

To set the default look, as Admin:

1. Open **Settings**, then **Appearance**.
2. Open the group **Layout**.
3. Choose **Default layout** and **Modern theme**.
4. Select **Save the layout**.

The settings are `layout` (`og` or `modern`, `og` by default), `modernTheme` (`organizr` by default) and `modernColourSet` (`dark`, the only colour set for now). Only the Admin can change them. A Co-Admin sees them but cannot save.

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
