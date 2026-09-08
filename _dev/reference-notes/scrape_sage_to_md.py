#!/usr/bin/env python3
import os
import re
import sys
import time
from urllib.parse import urljoin, urlparse

import requests
from bs4 import BeautifulSoup

BASE = "https://sage-docs.vercel.app"
START = "/docs"
OUTDIR = "sage-docs-md"


def slug_from_path(path: str) -> str:
    path = path.strip("/")
    if not path:
        return "index.md"
    if path.endswith("docs"):
        return os.path.join(path, "index.md")
    return path + ".md"


def clean_text(text: str) -> str:
    text = text.replace("\xa0", " ")
    text = re.sub(r"\n{3,}", "\n\n", text)
    return text.strip()


def node_to_md(node):
    name = getattr(node, "name", None)
    if name is None:
        return str(node)

    if name in ["h1", "h2", "h3", "h4", "h5", "h6"]:
        level = int(name[1])
        return f"{'#' * level} {node.get_text(' ', strip=True)}\n\n"

    if name == "p":
        return f"{node.get_text(' ', strip=True)}\n\n"

    if name in ["ul", "ol"]:
        lines = []
        for i, li in enumerate(node.find_all("li", recursive=False), start=1):
            prefix = f"{i}. " if name == "ol" else "- "
            lines.append(prefix + li.get_text(" ", strip=True))
        return "\n".join(lines) + "\n\n" if lines else ""

    if name == "pre":
        code = node.get_text("\n", strip=False)
        return f"```\n{code.rstrip()}\n```\n\n"

    if name == "code":
        return f"`{node.get_text(strip=True)}`"

    if name == "table":
        rows = []
        for tr in node.find_all("tr"):
            cells = [c.get_text(" ", strip=True) for c in tr.find_all(["th", "td"])]
            if cells:
                rows.append(cells)
        if not rows:
            return ""
        width = max(len(r) for r in rows)
        rows = [r + [""] * (width - len(r)) for r in rows]
        header = rows[0]
        body = rows[1:] if len(rows) > 1 else []
        md = "| " + " | ".join(header) + " |\n"
        md += "| " + " | ".join(["---"] * width) + " |\n"
        for r in body:
            md += "| " + " | ".join(r) + " |\n"
        return md + "\n"

    chunks = []
    for child in node.children:
        chunks.append(node_to_md(child))
    return "".join(chunks)


def extract_links(soup):
    links = set()
    for a in soup.select("a[href]"):
        href = a.get("href", "").strip()
        if not href or href.startswith("#"):
            continue
        full = urljoin(BASE, href)
        parsed = urlparse(full)
        if parsed.netloc != urlparse(BASE).netloc:
            continue
        if not parsed.path.startswith("/docs"):
            continue
        links.add(parsed.path)
    return links


def scrape_page(session, path):
    url = urljoin(BASE, path)
    r = session.get(url, timeout=30)
    r.raise_for_status()
    soup = BeautifulSoup(r.text, "html.parser")

    main = soup.find("main") or soup.body
    title = soup.title.get_text(" ", strip=True) if soup.title else path

    for tag in main.select("nav, aside, footer, script, style"):
        tag.decompose()

    content_parts = [f"# {title}\n\n", f"Source: {url}\n\n"]
    for child in main.find_all(recursive=False):
        content_parts.append(node_to_md(child))
    content = clean_text("".join(content_parts)) + "\n"
    return content, extract_links(soup)


def main():
    session = requests.Session()
    session.headers.update({"User-Agent": "Mozilla/5.0 (compatible; sage-docs-scraper/1.0)"})

    os.makedirs(OUTDIR, exist_ok=True)
    seen = set()
    queue = [START]

    while queue:
        path = queue.pop(0)
        if path in seen:
            continue
        seen.add(path)
        try:
            md, links = scrape_page(session, path)
        except Exception as e:
            print(f"Failed: {path} -> {e}", file=sys.stderr)
            continue

        rel = slug_from_path(path)
        dest = os.path.join(OUTDIR, rel)
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        with open(dest, "w", encoding="utf-8") as f:
            f.write(md)

        for link in sorted(links):
            if link not in seen and link not in queue:
                queue.append(link)
        time.sleep(0.2)

    readme = os.path.join(OUTDIR, "README.md")
    with open(readme, "w", encoding="utf-8") as f:
        f.write("# Sage Docs Markdown Export\n\n")
        f.write(f"Exported {len(seen)} pages from {BASE}/docs\n")

    print(f"Saved {len(seen)} pages into {OUTDIR}")


if __name__ == "__main__":
    main()