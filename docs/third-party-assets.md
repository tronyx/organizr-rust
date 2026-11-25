# 🎨 Pictures, fonts and translations from other projects

[← All documentation](README.md)

Organizr ships pictures, fonts and translations that other people made. This page lists them, where they come from, and the terms under which their makers publish them. It's for anyone who redistributes the image or wonders about a license.

## 🧭 Where the files come from

| Files | Source | How they get here |
|---|---|---|
| Pictures in `web/public/plugins/images/` | Organizr 2.1.5000, the PHP application that this project replaces | `web/tools/import-organizr-assets.py` for the folders `tabs`, `categories`, `groups`, `homepage` and `organizr`; copied by hand for the rest (see below) |
| Language files in `web/public/langpack/` | Organizr 2.1.5000 | `web/tools/import-organizr-langpacks.py` |
| Icon fonts in `web/src/icons/` | Organizr 2.1.5000, which bundles them from their projects | `web/tools/import-organizr-assets.py` |
| Fonts of the modern themes | The npm packages of Fontsource | `web/package.json`, bundled by the build |
| Interface icons (Lucide) | The npm package `@lucide/svelte` | `web/package.json`, bundled by the build |

A tab, a category or a group of an old install names its picture by path or icon name, so an install that you import keeps its pictures and icons.

## 🌄 Pictures

The folder `web/public/plugins/images` holds 216 files:

| Place | Files | What they are |
|---|---|---|
| `tabs/` | 168 | Logos of applications and services, for tabs |
| `categories/` | 1 | The picture of the category "Unsorted" |
| `groups/` | 22 | Pictures for groups of users |
| `homepage/` | 7 | Stand-in pictures of the homepage, such as "Poster coming soon" (`no-list.png`) |
| `organizr/` | 8 | Logos of Organizr |
| `plugins/` | 7 | The card pictures of the seven built-in plugins: Bookmark, Chat, Health Checks, Invites, PHP Mailer, Plex Libraries and Speed Test |
| `bookmark.png`, `default_user.png`, `login-register.jpg` | 3 | The bookmark picture, the picture of a user without one, and the default wallpaper of the login page and the lock screen |

All of them come from Organizr 2.1.5000. The six plugin pictures of the PHP plugins and the three loose files were copied from its `plugins/images` and plugin folders. `plugins/plexLibraries.png` is a copy of `tabs/plex.png`, because PHP has no built-in Plex Libraries plugin (D-121 in [the deviations](deviations.md)).

The pictures come with Organizr, which is published under the GNU General Public License, version 3. A logo of an application or a service is a trademark of its owner. Organizr uses it only to show a link to that application or service.

The server sends these pictures at the same paths as PHP Organizr, for example `plugins/images/tabs/plex.png`.

The icon of the browser tab, `web/public/favicon.svg`, and the animated art of the scene themes (`web/src/scenes/`) were made for this project, as inline drawings in its own code.

## 🌍 Language files

The folder `web/public/langpack` holds the 32 language files of Organizr, one for each language, with the English texts of the interface and their translations. The people of the Organizr community wrote the translations. They come with Organizr, under the GNU General Public License, version 3.

## 🔣 Icon fonts

The folder `web/src/icons` holds five icon fonts. For each font, the import script writes a style sheet with the names of the icons as PHP Organizr stores them.

| Family | Stored as | Project | Terms |
|---|---|---|---|
| Font Awesome 4.7 | `fontawesome::<name>` | Font Awesome by Dave Gandy | Font: SIL OFL 1.1. Code: MIT |
| Themify Icons | `themify::<name>` | Themify Icons by Themify | Font: SIL OFL 1.1. Code: MIT |
| Material Design Icons 1.8.36 | `materialize::<name>` | materialdesignicons.com | Community icons: SIL OFL 1.1. Icons by Google: Apache 2.0 |
| Simple Line Icons | `simpleline::<name>` | Simple Line Icons by Sabbir Ahmed and others | MIT |
| Weather Icons | `weathericon::<name>` | Weather Icons by Erik Flowers | Font: SIL OFL 1.1. Code: MIT |

The script also copies the icon list of PHP Organizr, with the names of the icons to show, to `crates/api/src/handlers/icons.json`. The route `GET /icon` sends it.

> [!NOTE]
> The source of Organizr holds these fonts without a license file. The terms in the table are the terms that each project publishes. Check them again before you publish a release.

## 🔤 Fonts of the modern themes

The modern layout loads these fonts from npm packages of [Fontsource](https://fontsource.org/), version 5.3.0. The build bundles the font files, so the page loads nothing from another site. Each package is published under the SIL Open Font License 1.1 (`OFL-1.1` in its `package.json`).

| Family | Package | Used by |
|---|---|---|
| Rubik | `@fontsource-variable/rubik` | The text of the interface |
| Plus Jakarta Sans | `@fontsource-variable/plus-jakarta-sans` | The Silo theme |
| Cinzel | `@fontsource-variable/cinzel` | Page headings of the Nocturne theme |
| Sedgwick Ave Display | `@fontsource/sedgwick-ave-display` | Page headings of the Riot theme |
| Pirata One | `@fontsource/pirata-one` | Page headings of the Tempest theme |
| Orbitron | `@fontsource-variable/orbitron` | Page headings of the Circuit theme |
| Share Tech Mono | `@fontsource/share-tech-mono` | Badges and code of the Circuit theme |
| Fredoka | `@fontsource-variable/fredoka` | The text of the Prism theme |

A scene theme loads its font only while that theme is on.

## 🔘 Interface icons

The buttons and menus of the interface draw their icons with [Lucide](https://lucide.dev/) (`@lucide/svelte`), published under the ISC license.

## 🔄 How to update the files

Do this only when the reference version of Organizr changes:

1. Build the image of the parity reference:

   ```bash
   docker compose -f parity/oracle/compose.yml build
   ```

2. In the root of the repository, run both import scripts:

   ```bash
   python web/tools/import-organizr-assets.py
   python web/tools/import-organizr-langpacks.py
   ```

3. Run the tests of the web interface and of the server (see [CONTRIBUTING](../CONTRIBUTING.md)).
4. Commit the changed files.

The fonts of the modern themes and Lucide update with the other web libraries (see [Releases and images](releases.md)).
