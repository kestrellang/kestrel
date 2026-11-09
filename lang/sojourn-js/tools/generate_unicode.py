#!/usr/bin/env python3
"""Writes the ID_Start / ID_Continue range tables the lexer reads.

Each table is little-endian Int32 pairs (lo, hi), sorted and merged, for
binary search. Derived from Python's unicodedata (the Unicode version is
printed), following UAX #31's definitions of ID_Start and ID_Continue.

Usage: python3 tools/generate_unicode.py   (from lang/sojourn-js)
"""
import struct
import sys
import unicodedata
from pathlib import Path

START_CATS = {"Lu", "Ll", "Lt", "Lm", "Lo", "Nl"}
CONTINUE_CATS = START_CATS | {"Mn", "Mc", "Nd", "Pc"}
OTHER_ID_START = {0x1885, 0x1886, 0x2118, 0x212E, 0x309B, 0x309C}
OTHER_ID_CONTINUE = {0x00B7, 0x0387, 0x1369, 0x136A, 0x136B, 0x136C, 0x136D,
                     0x136E, 0x136F, 0x1370, 0x1371, 0x19DA}
# Pattern_Syntax characters that would otherwise qualify.
EXCLUDED = {0x2E2F}


def ranges(pred):
    out = []
    for cp in range(0x110000):
        if pred(cp):
            if out and out[-1][1] == cp - 1:
                out[-1][1] = cp
            else:
                out.append([cp, cp])
    return out


def is_start(cp):
    if cp in EXCLUDED:
        return False
    return unicodedata.category(chr(cp)) in START_CATS or cp in OTHER_ID_START


def is_continue(cp):
    if cp in EXCLUDED:
        return False
    return (unicodedata.category(chr(cp)) in CONTINUE_CATS
            or cp in OTHER_ID_START or cp in OTHER_ID_CONTINUE)


def write(table, path):
    with open(path, "wb") as f:
        for lo, hi in table:
            f.write(struct.pack("<ii", lo, hi))


def main():
    out = Path(__file__).resolve().parent.parent / "src" / "data"
    out.mkdir(exist_ok=True)
    start = ranges(is_start)
    cont = ranges(is_continue)
    write(start, out / "id_start.bin")
    write(cont, out / "id_continue.bin")
    print(f"Unicode {unicodedata.unidata_version}: "
          f"{len(start)} ID_Start ranges, {len(cont)} ID_Continue ranges",
          file=sys.stderr)


if __name__ == "__main__":
    main()
