"""Copies the language files of Organizr 2.1.5000 into this project.

PHP Organizr translates its page in the browser with the files `js/langpack/*.json`, one
for each language, named `<code>[<name>].json`. This application translates its page with
the same files (see `web/src/lib/i18n.svelte.ts`).

The source is the image of the parity reference, which holds the exact upstream source:

    docker compose -f parity/oracle/compose.yml build
    python web/tools/import-organizr-langpacks.py

The script writes:

- `web/public/langpack/<code>.json`: the English texts and their translations of each
  language, served at `langpack/<code>.json`.
- `web/src/lib/languages.ts`: the code and the name of each language, sorted by name as
  PHP sorts them.

Run it again only when the reference changes. Its results are committed.
"""

import html
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

IMAGE = "organizr-parity-oracle"
SOURCE = "/var/www/html/js/langpack"
REPOSITORY = Path(__file__).resolve().parent.parent.parent
TARGET = REPOSITORY / "web" / "public" / "langpack"
LIST = REPOSITORY / "web" / "src" / "lib" / "languages.ts"


def docker(*arguments: str) -> str:
    return subprocess.run(
        ["docker", *arguments], check=True, capture_output=True, text=True
    ).stdout.strip()


def main() -> None:
    container = docker("create", IMAGE)
    try:
        with tempfile.TemporaryDirectory() as folder:
            copied = Path(folder) / "langpack"
            docker("cp", f"{container}:{SOURCE}", str(copied))
            if TARGET.exists():
                shutil.rmtree(TARGET)
            TARGET.mkdir(parents=True)
            languages = []
            for file in sorted(copied.glob("*.json")):
                if "[" not in file.name:
                    continue
                code = file.name.split("[")[0]
                name = html.unescape(file.name[file.name.index("[") + 1 : file.name.rindex("]")])
                tokens = json.loads(file.read_text(encoding="utf-8")).get("token", {})
                # Only the pairs of texts: the page uses nothing else of the files.
                pairs = {
                    english: translated
                    for english, translated in tokens.items()
                    if isinstance(english, str) and isinstance(translated, str)
                }
                (TARGET / f"{code}.json").write_text(
                    json.dumps(pairs, ensure_ascii=False, separators=(",", ":"), sort_keys=True),
                    encoding="utf-8",
                    newline="\n",
                )
                languages.append((name, code))
    finally:
        docker("rm", container)

    languages.sort()
    lines = [
        "// The languages of the interface: the language files of PHP Organizr (js/langpack),",
        "// with the code of each one and its name, sorted by name as PHP sorts them. The files",
        "// are in `public/langpack`, one for each code. Written by",
        "// `web/tools/import-organizr-langpacks.py`.",
        "",
        "export const LANGUAGES: { code: string; name: string }[] = [",
    ]
    for name, code in languages:
        lines.append(f"  {{ code: '{code}', name: '{name}' }},")
    lines += ["];", ""]
    LIST.write_text("\n".join(lines), encoding="utf-8", newline="\n")
    print(f"{len(languages)} languages")


if __name__ == "__main__":
    main()
