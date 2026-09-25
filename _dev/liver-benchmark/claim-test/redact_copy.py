#!/usr/bin/env python3
"""Copy text files into the repository with personal paths redacted.

Usage: python3 redact_copy.py DEST_DIR FILE [FILE ...]

The user part of a home path becomes `<redacted>/`, with the rule of
_dev/liver-benchmark/README.md. Temporary work folders
(/private/tmp/..., /tmp/..., /var/folders/...) up to the last path
separator are replaced the same way. Prints the number of replacements per
file. Python 3 standard library only.
"""
import os
import re
import sys

HOME = re.compile(rb"(?i)(?:[A-Za-z]\\?:)?(?:\\+|/)Users(?:\\+|/)[^\\/\s\"'<>]+(?:\\+|/)")
HOME_LINUX = re.compile(rb"/home/[^/\s\"'<>]+/")
TMP = re.compile(rb"(?:/private)?/(?:tmp|var/folders)/[^\s\"'<>]*/")


def main():
    dest = sys.argv[1]
    os.makedirs(dest, exist_ok=True)
    for src in sys.argv[2:]:
        data = open(src, "rb").read()
        n = 0
        for rx in (TMP, HOME, HOME_LINUX):
            data, k = rx.subn(b"<redacted>/", data)
            n += k
        out = os.path.join(dest, os.path.basename(src))
        open(out, "wb").write(data)
        print(f"{src} -> {out}: {n} replacements")


if __name__ == "__main__":
    main()
