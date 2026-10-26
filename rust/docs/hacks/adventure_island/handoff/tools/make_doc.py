"""Writes the appendices of the hack's document from the sources (everything from "### A." on).

Usage: python -I tools/make_doc.py <zaxnes>/rust/docs/hacks/adventure_island/WIDESCREEN-HACK.md
"""

import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
APPENDICES = [('A', 'src/wide.s', 'asm'), ('B', 'tools/build.py', 'python'), ('C', 'tools/make_bps.py', 'python')]


def main():
    path = sys.argv[1]
    text = open(path, encoding='utf-8').read()
    body = text.split('\n### A. ')[0].rstrip('\n') + '\n'
    for letter, source, language in APPENDICES:
        code = open(os.path.join(ROOT, source), encoding='utf-8').read().rstrip('\n')
        body += f'\n### {letter}. `{source}`\n\n```{language}\n{code}\n```\n'
    open(path, 'w', encoding='utf-8', newline='\n').write(body)


if __name__ == '__main__':
    main()
