#!/usr/bin/env python3
"""Build docs/EAW-paper.pdf from docs/EAW-paper.md with headless Chrome.

Needs: python `markdown` package and Google Chrome (set CHROME to override the path).
Usage: python3 scripts/build_paper.py
"""
import os
import subprocess
import sys
from pathlib import Path

import markdown

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
CHROME = os.environ.get("CHROME", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
CSS = """
body{font:11pt/1.5 Georgia,serif;max-width:46em;margin:2em auto;color:#1a202c}
h1{font-size:20pt;margin-bottom:.2em} h2{font-size:14pt;margin-top:1.6em;border-bottom:1px solid #cbd5e0}
h3{font-size:12pt} img{max-width:100%;margin:.6em 0} code{font:9.5pt Menlo,monospace;background:#edf2f7;padding:0 .2em}
pre{background:#edf2f7;padding:.6em;overflow:auto;font-size:9pt} table{border-collapse:collapse;font-size:9.5pt;margin:.8em 0}
td,th{border:1px solid #cbd5e0;padding:.25em .5em} blockquote{border-left:3px solid #cbd5e0;margin-left:0;padding-left:1em;color:#4a5568}
@page{margin:18mm}
"""


def main():
    md = (DOCS / "EAW-paper.md").read_text()
    body = markdown.markdown(md, extensions=["tables", "fenced_code", "sane_lists"])
    html = f"<!doctype html><meta charset='utf-8'><title>EAW paper</title><style>{CSS}</style>{body}"
    (DOCS / "EAW-paper.html").write_text(html)
    if not Path(CHROME).exists():
        sys.exit(f"Chrome not found at {CHROME}")
    subprocess.run(
        [
            CHROME,
            "--headless=new",
            "--disable-gpu",
            "--no-pdf-header-footer",
            f"--print-to-pdf={DOCS / 'EAW-paper.pdf'}",
            (DOCS / "EAW-paper.html").as_uri(),
        ],
        check=True,
        capture_output=True,
    )
    print("wrote docs/EAW-paper.pdf")


if __name__ == "__main__":
    main()
