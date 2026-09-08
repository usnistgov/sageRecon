#!/usr/bin/env python3
import argparse
import os
import re
from pathlib import Path


def natural_key(text: str):
    return [int(c) if c.isdigit() else c.lower() for c in re.split(r'(\d+)', text)]


def collect_md_files(root: Path):
    files = [p for p in root.rglob('*.md') if p.is_file()]
    return sorted(files, key=lambda p: natural_key(str(p.relative_to(root))))


def strip_yaml_frontmatter(text: str) -> str:
    if text.startswith('---\n'):
        parts = text.split('\n---\n', 1)
        if len(parts) == 2:
            return parts[1].lstrip()
    return text


def normalize_heading_levels(text: str, bump: int = 1) -> str:
    out = []
    in_code = False
    for line in text.splitlines():
        if line.strip().startswith('```'):
            in_code = not in_code
            out.append(line)
            continue
        if not in_code and re.match(r'^#{1,6}\s', line):
            hashes, rest = line.split(' ', 1)
            level = min(len(hashes) + bump, 6)
            out.append('#' * level + ' ' + rest)
        else:
            out.append(line)
    return '\n'.join(out).strip()


def merge_folder(input_dir: Path, output_file: Path, title: str | None):
    files = collect_md_files(input_dir)
    if not files:
        raise SystemExit(f'No markdown files found in {input_dir}')

    chunks = []
    doc_title = title or f'{input_dir.name} combined'
    chunks.append(f'# {doc_title}\n')
    chunks.append(f'_Source folder: `{input_dir}`_\n')

    for md_file in files:
        rel = md_file.relative_to(input_dir)
        text = md_file.read_text(encoding='utf-8', errors='ignore')
        text = strip_yaml_frontmatter(text).strip()
        text = normalize_heading_levels(text, bump=1)
        section_title = str(rel.with_suffix('')).replace('\\', '/')
        chunks.append(f'\n## {section_title}\n')
        if text:
            chunks.append(text + '\n')
        else:
            chunks.append('_Empty file._\n')

    output_file.parent.mkdir(parents=True, exist_ok=True)
    output_file.write_text('\n'.join(chunks).strip() + '\n', encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description='Merge a folder of markdown files into one markdown file.')
    parser.add_argument('input_dir', help='Folder containing markdown files')
    parser.add_argument('-o', '--output', default='combined.md', help='Output markdown filename')
    parser.add_argument('--title', default=None, help='Top-level title for the combined markdown file')
    args = parser.parse_args()

    merge_folder(Path(args.input_dir), Path(args.output), args.title)


if __name__ == '__main__':
    main()
