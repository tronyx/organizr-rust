# Pictures and icon fonts from other projects

Organizr ships pictures and icon fonts that other people made. This page lists them, where they come from, and under which terms their makers publish them.

All of these files come from the source of Organizr 2.1.5000, which is the PHP application that this project replaces. The script `web/tools/import-organizr-assets.py` copies them from that source. A tab, a category or a group of an old install names these files, so an install that you import keeps its pictures.

## Pictures

The folder `web/public/plugins/images` holds 199 pictures in four folders:

| Folder | What it holds |
|---|---|
| `tabs` | The logos of 168 applications and services, for tabs |
| `categories` | The picture of the category "Unsorted" |
| `groups` | 22 pictures for groups of users |
| `organizr` | 8 logos of Organizr |

The pictures come with Organizr, which is published under the GNU General Public License, version 3. A logo of an application or a service is a mark of its owner. Organizr uses it only to show a link to that application or service.

The server sends these pictures at the same paths as PHP Organizr does, for example `plugins/images/tabs/plex.png`.

## Language files

The folder `web/public/langpack` holds the 32 language files of Organizr, one for each language, with the English texts of the interface and their translations. The people of the Organizr community wrote the translations. They come with Organizr, which is published under the GNU General Public License, version 3. The script `web/tools/import-organizr-langpacks.py` copies them from the same source as the pictures.

## Icon fonts

The folder `web/src/icons` holds five icon fonts. For each font, the script writes a style sheet with the names of the icons as PHP Organizr stores them.

| Family | Stored as | Project | Terms |
|---|---|---|---|
| Font Awesome 4.7 | `fontawesome::<name>` | Font Awesome by Dave Gandy | Font: SIL OFL 1.1. Code: MIT |
| Themify Icons | `themify::<name>` | Themify Icons by Themify | Font: SIL OFL 1.1. Code: MIT |
| Material Design Icons 1.8.36 | `materialize::<name>` | materialdesignicons.com | Community icons: SIL OFL 1.1. Icons by Google: Apache 2.0 |
| Simple Line Icons | `simpleline::<name>` | Simple Line Icons by Sabbir Ahmed and others | MIT |
| Weather Icons | `weathericon::<name>` | Weather Icons by Erik Flowers | Font: SIL OFL 1.1. Code: MIT |

The script also copies the icon list of PHP Organizr, with the names of the icons to show, to `crates/api/src/handlers/icons.json`. The route `GET /icon` sends it.

The source of Organizr holds these fonts without a licence file. The terms in the table are the terms that each project publishes. Make sure that they are correct before you publish a release.

## How to update the files

Do this only when the reference version of Organizr changes:

1. Build the image of the parity reference: `docker compose -f parity/oracle/compose.yml build`.
2. Run `python web/tools/import-organizr-assets.py` in the root of the repository.
3. Run the tests of the web interface and of the server.
4. Commit the changed files.
