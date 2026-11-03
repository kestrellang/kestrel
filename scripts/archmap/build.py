#!/usr/bin/env python3
"""Build the interactive compiler map: run the extractor, inline its JSON into
template.html, and write a single self-contained HTML file.

    python3 scripts/archmap/build.py [repo_root] [-o out.html]

The output has no runtime dependencies besides Google Fonts, so it can be
opened locally or published as-is.
"""

import argparse
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('root', nargs='?', default='.', help='repo checkout to map (default: cwd)')
    ap.add_argument('-o', '--out', default='archmap.html')
    ap.add_argument('--json', help='also write the raw extracted JSON here')
    args = ap.parse_args()

    data = subprocess.run([sys.executable, os.path.join(HERE, 'extract.py'), args.root],
                          check=True, capture_output=True, text=True).stdout
    if args.json:
        with open(args.json, 'w') as f:
            f.write(data)
    template = open(os.path.join(HERE, 'template.html'), encoding='utf-8').read()
    # `</` would end the <script> element early; JSON allows the escaped form.
    html = template.replace('/*ARCHMAP_DATA*/', data.replace('</', '<\\/'))
    with open(args.out, 'w', encoding='utf-8') as f:
        f.write(html)
    print(f'wrote {args.out} ({len(html) // 1024} KB)', file=sys.stderr)


if __name__ == '__main__':
    main()
