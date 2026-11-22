"""Checks the project's Markdown and the copy of docs/ in the wiki.

Every relative link in the README, the pages in docs/ and CONTRIBUTING
must name a file that exists, and an #anchor must match a heading in it,
by the rules GitHub uses to make a heading's anchor. The wiki copy made by
docs_to_wiki.py must keep every page, rewrite the links as the wiki needs
them and change nothing else. Standard library only:

    python3 -m unittest discover -s .github/scripts
"""

import importlib.util
import re
import tempfile
import unicodedata
import unittest
from pathlib import Path
from types import ModuleType
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent.parent
"""The repository's root folder."""

REPO = "https://github.com/tronyx/organizr-rust"
"""This repository on GitHub."""

BRANCH = "develop"
"""The branch that the wiki follows."""


def load_docs_to_wiki() -> ModuleType:
    """docs_to_wiki.py as a module: it's a script, not part of a package."""
    spec = importlib.util.spec_from_file_location("docs_to_wiki", Path(__file__).with_name("docs_to_wiki.py"))
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


docs_to_wiki = load_docs_to_wiki()


def markdown_files(root: Path) -> list[Path]:
    """The Markdown files that readers follow links in."""
    found = [root / "README.md", root / "CONTRIBUTING.md"]
    found += sorted((root / "docs").glob("**/*.md"))
    return [path for path in found if path.is_file()]


def without_code(text: str) -> str:
    """text without its fenced code blocks and inline code, which hold no
    links or headings."""
    text = re.sub(r"^ *```.*?^ *```", "", text, flags=re.MULTILINE | re.DOTALL)
    return re.sub(r"`[^`\n]*`", "", text)


def github_slug(heading: str) -> str:
    """The anchor GitHub gives a heading: lowercased, with letters, marks,
    numbers, "-" and "_" kept, spaces turned into hyphens, and everything
    else dropped."""
    kept = []
    for char in heading.strip().lower():
        if char == " ":
            kept.append("-")
        elif char in "-_" or unicodedata.category(char)[0] in "LMN":
            kept.append(char)
    return "".join(kept)


def anchors(path: Path) -> set[str]:
    """The anchors of the headings of a Markdown file. A repeated heading
    gets "-1", "-2" and so on, as on GitHub."""
    seen: dict[str, int] = {}
    found: set[str] = set()
    text = without_code(path.read_text(encoding="utf-8"))
    for heading in re.findall(r"^#{1,6} +(.+?) *#* *$", text, re.MULTILINE):
        slug = github_slug(heading)
        count = seen.get(slug, 0)
        found.add(slug if count == 0 else f"{slug}-{count}")
        seen[slug] = count + 1
    return found


def broken_links(root: Path) -> list[str]:
    """Every relative link under root that leads nowhere, as "file: link: why"."""
    problems = []
    for source in markdown_files(root):
        text = without_code(source.read_text(encoding="utf-8"))
        links = re.findall(r"\]\(<?([^)\s>]+)>?(?:\s+\"[^\"]*\")?\)", text)
        links += re.findall(r"^\[[^\]]+\]:\s*(\S+)", text, re.MULTILINE)
        for link in links:
            if re.match(r"^[a-z][a-z+.-]*:", link):
                continue
            path, _, anchor = link.partition("#")
            target = (source.parent / unquote(path)).resolve() if path else source
            where = source.relative_to(root).as_posix()
            if not target.exists():
                problems.append(f"{where}: {link}: {path} doesn't exist")
            elif anchor and target.suffix == ".md" and unquote(anchor) not in anchors(target):
                problems.append(f"{where}: {link}: no heading for #{anchor}")
    return problems


class Links(unittest.TestCase):
    def test_headings_get_the_anchors_github_gives_them(self) -> None:
        self.assertEqual(github_slug("Move from PHP Organizr"), "move-from-php-organizr")
        self.assertEqual(github_slug("Colours: CSS variables"), "colours-css-variables")
        self.assertEqual(github_slug("`ORGANIZR_TRUSTED_PROXIES`"), "organizr_trusted_proxies")

    def test_every_link_in_the_docs_leads_somewhere(self) -> None:
        self.assertEqual(broken_links(ROOT), [])


class WikiCopy(unittest.TestCase):
    BLOB = f"{REPO}/blob/{BRANCH}/"

    def test_links_are_rewritten_for_the_wiki(self) -> None:
        cases = {
            "email.md": "email",
            "email.md#smtp": "email#smtp",
            "README.md": "Home",
            "#colours": "#colours",
            "https://example.com/a.md": "https://example.com/a.md",
            "../README.md": self.BLOB + "README.md",
            "../docker/examples/compose.postgres.yaml": self.BLOB + "docker/examples/compose.postgres.yaml",
            "pictures/login.png": "pictures/login.png",
        }
        for target, expected in cases.items():
            with self.subTest(target=target):
                self.assertEqual(docs_to_wiki.wiki_target(target, "docs", self.BLOB), expected)

    def test_code_is_left_alone(self) -> None:
        page = "See [mail](email.md) and `[x](email.md)`.\n\n```\n[y](email.md)\n```\n"
        self.assertEqual(
            docs_to_wiki.wiki_page(page, "docs", self.BLOB),
            "See [mail](email) and `[x](email.md)`.\n\n```\n[y](email.md)\n```\n",
        )

    def test_the_copy_holds_every_page_and_only_them(self) -> None:
        with tempfile.TemporaryDirectory() as wiki:
            wiki_dir = Path(wiki)
            (wiki_dir / ".git").mkdir()
            (wiki_dir / "Old-page.md").write_text("A page that docs/ no longer has.", encoding="utf-8")
            docs_to_wiki.sync(ROOT / "docs", wiki_dir, REPO, BRANCH)

            pages = {path.name for path in wiki_dir.glob("*.md")}
            expected = {docs_to_wiki.page_name(path.name) + ".md" for path in (ROOT / "docs").glob("*.md")}
            self.assertEqual(pages, expected | {"_Footer.md"})
            self.assertIn("Home.md", pages)
            self.assertTrue((wiki_dir / ".git").is_dir())
            footer = (wiki_dir / "_Footer.md").read_text(encoding="utf-8")
            self.assertIn(f"{REPO}/tree/{BRANCH}/docs", footer)


if __name__ == "__main__":
    unittest.main()
