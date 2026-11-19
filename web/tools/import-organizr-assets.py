"""Copies the pictures and the icon fonts of Organizr 2.1.5000 into this project.

A tab, a category or a group of PHP Organizr has a picture: a file such as
`plugins/images/tabs/plex.png`, or an icon such as `fontawesome::cog`. An install that is
imported keeps these values, so this application ships the same files and the same icons.

The source is the image of the parity reference, which holds the exact upstream source:

    docker compose -f parity/oracle/compose.yml build
    python web/tools/import-organizr-assets.py

The script writes:

- `web/public/plugins/images/<folder>/`: the pictures, served at the paths that PHP uses.
- `web/src/icons/<family>.css` and `web/src/icons/fonts/`: one style sheet and one font
  for each family of icons.
- `web/src/lib/iconNames.json`: the names of all icons, for the picture chooser.
- `crates/api/src/handlers/bundled_tab_images.txt`: the names of the pictures for tabs,
  for the image routes of the API.
- `crates/import/src/bundled_images.txt`: the paths of all pictures, for the importer.

Run it again only when the reference changes. Its results are committed.
"""

import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

IMAGE = "organizr-parity-oracle"
SOURCE = "/var/www/html"
REPOSITORY = Path(__file__).resolve().parent.parent.parent
WEB = REPOSITORY / "web"

# The folders of pictures that the application uses.
PICTURE_FOLDERS = ("tabs", "categories", "groups", "organizr")
# Files in those folders that are no pictures.
NOT_A_PICTURE = {"index.html", ".DS_Store"}

# Each family of icons: the word before `::` in a stored value, the prefix of its class
# names in the upstream style sheet, that style sheet, the font, and a line for the file.
FAMILIES = (
    {
        "word": "fontawesome",
        "prefix": "fa-",
        "sheet": "css/dark.css",
        "font": "less/icons/font-awesome/fonts/fontawesome-webfont.woff2",
        "about": "Font Awesome 4.7 by Dave Gandy. Font: SIL OFL 1.1. Code: MIT.",
    },
    {
        "word": "themify",
        "prefix": "ti-",
        "sheet": "css/dark.css",
        "font": "less/icons/themify-icons/fonts/themify.woff",
        "about": "Themify Icons by Themify. Font: SIL OFL 1.1. Code: MIT.",
    },
    {
        "word": "materialize",
        "prefix": "mdi-",
        "sheet": "css/icons/material-design-iconic-font/css/materialdesignicons.min.css",
        "font": "css/icons/material-design-iconic-font/fonts/materialdesignicons-webfont.woff2",
        "about": "Material Design Icons 1.8.36 from materialdesignicons.com. Community icons: SIL OFL 1.1. Icons by Google: Apache 2.0.",
    },
    {
        "word": "simpleline",
        "prefix": "icon-",
        "sheet": "css/dark.css",
        "font": "less/icons/simple-line-icons/fonts/Simple-Line-Icons.woff2",
        "about": "Simple Line Icons by Sabbir Ahmed and others. MIT.",
        # The upstream style sheet uses the same prefix for other fonts, so only the names
        # of the icon list are taken.
        "listed_only": True,
    },
    {
        "word": "weathericon",
        "prefix": "wi-",
        "sheet": "css/dark.css",
        "font": "less/icons/weather-icons/font/weathericons-regular-webfont.woff2",
        "about": "Weather Icons by Erik Flowers. Font: SIL OFL 1.1. Code: MIT.",
    },
)


def docker(*arguments):
    result = subprocess.run(["docker", *arguments], capture_output=True, text=True)
    if result.returncode != 0:
        sys.exit(f"docker {' '.join(arguments)} failed:\n{result.stderr}")
    return result.stdout.strip()


def glyphs(sheet, prefix):
    """The icons of a style sheet: name to the character that the font draws it with."""
    found = {}
    rule = re.compile(
        r"((?:\." + re.escape(prefix) + r"[A-Za-z0-9_-]+:before\s*,?\s*)+)"
        r"\{\s*content:\s*[\"']\\([0-9A-Fa-f]{3,6})[\"'];?\s*\}"
    )
    name = re.compile(r"\." + re.escape(prefix) + r"([A-Za-z0-9_-]+):before")
    for match in rule.finditer(sheet):
        for icon in name.findall(match.group(1)):
            found.setdefault(icon, match.group(2).lower())
    return found


def main():
    container = docker("create", IMAGE)
    try:
        with tempfile.TemporaryDirectory() as folder:
            work = Path(folder)
            import_pictures(container, work)
            import_icons(container, work)
    finally:
        docker("rm", container)


def import_pictures(container, work):
    target = WEB / "public" / "plugins" / "images"
    if target.exists():
        shutil.rmtree(target)
    for folder in PICTURE_FOLDERS:
        copied = work / f"pictures-{folder}"
        docker("cp", f"{container}:{SOURCE}/plugins/images/{folder}", str(copied))
        (target / folder).mkdir(parents=True)
        for file in sorted(copied.iterdir()):
            if file.is_file() and file.name not in NOT_A_PICTURE:
                shutil.copyfile(file, target / folder / file.name)
        print(f"{folder}: {len(list((target / folder).iterdir()))} pictures")

    names = sorted(file.name for file in (target / "tabs").iterdir())
    listing = REPOSITORY / "crates" / "api" / "src" / "handlers" / "bundled_tab_images.txt"
    listing.write_text("\n".join(names) + "\n", encoding="utf-8", newline="\n")

    # Every picture, by the path that PHP uses, for the importer: it must know which
    # pictures of an old install the application does not have.
    paths = sorted(
        file.relative_to(target.parents[1]).as_posix()
        for file in target.rglob("*")
        if file.is_file()
    )
    listing = REPOSITORY / "crates" / "import" / "src" / "bundled_images.txt"
    listing.write_text("\n".join(paths) + "\n", encoding="utf-8", newline="\n")


def import_icons(container, work):
    target = WEB / "src" / "icons"
    fonts = target / "fonts"
    fonts.mkdir(parents=True, exist_ok=True)

    docker("cp", f"{container}:{SOURCE}/js/icons.json", str(work / "icons.json"))
    listed = {}
    for group in json.loads((work / "icons.json").read_text(encoding="utf-8")):
        for icon in group["children"]:
            word, name = icon["id"].split("::", 1)
            listed.setdefault(word, []).append(name)

    names = {}
    for family in FAMILIES:
        word = family["word"]
        sheet_file = work / f"{word}.css"
        docker("cp", f"{container}:{SOURCE}/{family['sheet']}", str(sheet_file))
        found = glyphs(sheet_file.read_text(encoding="utf-8", errors="replace"), family["prefix"])
        if family.get("listed_only"):
            found = {name: found[name] for name in listed[word] if name in found}
        # The icon list of upstream names a few icons that its own style sheet does not have.
        # It also names some icons twice. Each name is kept once, in the order of the list.
        names[word] = list(dict.fromkeys(name for name in listed[word] if name in found))

        font = Path(family["font"]).name
        docker("cp", f"{container}:{SOURCE}/{family['font']}", str(fonts / font))
        kind = "woff2" if font.endswith(".woff2") else "woff"
        lines = [
            f"/* {family['about']} */",
            "/* The font and the names are those of Organizr 2.1.5000. */",
            "/* Made by web/tools/import-organizr-assets.py. Do not change this file by hand. */",
            "@font-face {",
            f"  font-family: 'organizr-{word}';",
            f"  src: url('./fonts/{font}') format('{kind}');",
            "  font-display: block;",
            "}",
            f".oi-{word} {{",
            f"  font-family: 'organizr-{word}';",
            "}",
        ]
        lines += [
            f".{family['prefix']}{name}::before{{content:'\\{code}'}}"
            for name, code in sorted(found.items())
        ]
        (target / f"{word}.css").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
        print(f"{word}: {len(found)} icons, {len(names[word])} of them in the icon list")

    (WEB / "src" / "lib" / "iconNames.json").write_text(
        json.dumps(names, separators=(",", ":")) + "\n", encoding="utf-8", newline="\n"
    )


if __name__ == "__main__":
    main()
