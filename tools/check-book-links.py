#!/usr/bin/env python3
"""Check files and fragment targets in the actual generated mdBook output."""

from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit


class Page(HTMLParser):
    def __init__(self, path):
        super().__init__()
        self.ids = set()
        self.links = []
        self.feed(path.read_text())

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.add(attrs["id"])
        for key in ("href", "src"):
            if attrs.get(key):
                self.links.append(attrs[key])


def main():
    root = Path(__file__).resolve().parent.parent / "target/book"
    pages = {path.resolve(): Page(path) for path in root.rglob("*.html")}
    if not pages:
        raise SystemExit("No generated HTML; run mdbook build first")
    errors = []
    for path, page in pages.items():
        for link in page.links:
            url = urlsplit(link)
            if url.scheme or url.netloc:
                continue
            if url.path.startswith("/"):
                target = (root / unquote(url.path).lstrip("/")).resolve()
            else:
                target = (path.parent / unquote(url.path)).resolve() if url.path else path
            if target.is_dir():
                target /= "index.html"
            if not target.exists():
                errors.append(f"{path.relative_to(root)}: missing {link}")
            elif url.fragment and target in pages:
                if unquote(url.fragment) not in pages[target].ids:
                    errors.append(f"{path.relative_to(root)}: missing fragment {link}")
    if errors:
        raise SystemExit("\n".join(errors))
    print(f"Checked local links, images, and fragments in {len(pages)} HTML pages")


if __name__ == "__main__":
    main()
