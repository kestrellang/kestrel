#!/usr/bin/env python3
"""Generates src/unicode_props.ks, the tables behind \\p{...} in regexps.

Usage: tools/gen_unicode_props.py UCD_DIR

UCD_DIR holds the Unicode Character Database files (UnicodeData.txt is not
needed; DerivedGeneralCategory.txt, Scripts.txt, ScriptExtensions.txt,
PropList.txt, DerivedCoreProperties.txt, DerivedBinaryProperties.txt,
DerivedNormalizationProps.txt, emoji-data.txt, PropertyAliases.txt and
PropertyValueAliases.txt are), e.g. from
https://www.unicode.org/Public/17.0.0/ucd/.

Each table is a set of code point ranges, encoded as base-36 deltas so the
generated source stays small; names map to table ids through lookup
strings of the form "|name:id|".
"""
import os, re, sys

MAX = 0x10FFFF

# ECMA-262 table-binary-unicode-properties.
BINARY = [
    "ASCII", "ASCII_Hex_Digit", "Alphabetic", "Any", "Assigned", "Bidi_Control", "Bidi_Mirrored",
    "Case_Ignorable", "Cased", "Changes_When_Casefolded", "Changes_When_Casemapped",
    "Changes_When_Lowercased", "Changes_When_NFKC_Casefolded", "Changes_When_Titlecased",
    "Changes_When_Uppercased", "Dash", "Default_Ignorable_Code_Point", "Deprecated", "Diacritic",
    "Emoji", "Emoji_Component", "Emoji_Modifier", "Emoji_Modifier_Base", "Emoji_Presentation",
    "Extended_Pictographic", "Extender", "Grapheme_Base", "Grapheme_Extend", "Hex_Digit",
    "IDS_Binary_Operator", "IDS_Trinary_Operator", "ID_Continue", "ID_Start", "Ideographic",
    "Join_Control", "Logical_Order_Exception", "Lowercase", "Math", "Noncharacter_Code_Point",
    "Pattern_Syntax", "Pattern_White_Space", "Quotation_Mark", "Radical", "Regional_Indicator",
    "Sentence_Terminal", "Soft_Dotted", "Terminal_Punctuation", "Unified_Ideograph", "Uppercase",
    "Variation_Selector", "White_Space", "XID_Continue", "XID_Start",
]

GC_GROUPS = {
    "L": ["Lu", "Ll", "Lt", "Lm", "Lo"], "LC": ["Lu", "Ll", "Lt"], "M": ["Mn", "Mc", "Me"],
    "N": ["Nd", "Nl", "No"], "P": ["Pc", "Pd", "Ps", "Pe", "Pi", "Pf", "Po"],
    "S": ["Sm", "Sc", "Sk", "So"], "Z": ["Zs", "Zl", "Zp"], "C": ["Cc", "Cf", "Cs", "Co", "Cn"],
}


def records(path):
    """(lo, hi, fields) for each data line of a UCD file."""
    for line in open(path, encoding="utf-8"):
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        fields = [f.strip() for f in line.split(";")]
        cps = fields[0].split("..")
        lo = int(cps[0], 16)
        hi = int(cps[-1], 16)
        yield lo, hi, fields[1:]


def merge(ranges):
    out = []
    for lo, hi in sorted(ranges):
        if out and lo <= out[-1][1] + 1:
            out[-1][1] = max(out[-1][1], hi)
        else:
            out.append([lo, hi])
    return out


def complement(ranges):
    out, nxt = [], 0
    for lo, hi in merge(ranges):
        if lo > nxt:
            out.append([nxt, lo - 1])
        nxt = hi + 1
    if nxt <= MAX:
        out.append([nxt, MAX])
    return out


def b36(n):
    digits = "0123456789abcdefghijklmnopqrstuvwxyz"
    s = ""
    while True:
        s = digits[n % 36] + s
        n //= 36
        if n == 0:
            return s


def encode(ranges):
    vals, prev = [], -1
    for lo, hi in merge(ranges):
        vals.append(lo - prev - 1)
        vals.append(hi - lo)
        prev = hi
    return ",".join(b36(v) for v in vals)


def aliases(ucd):
    """Value aliases per property short name, and property aliases."""
    values = {}
    for line in open(os.path.join(ucd, "PropertyValueAliases.txt"), encoding="utf-8"):
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        f = [x.strip() for x in line.split(";")]
        values.setdefault(f[0], []).append(f[1:])
    props = {}
    for line in open(os.path.join(ucd, "PropertyAliases.txt"), encoding="utf-8"):
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        f = [x.strip() for x in line.split(";")]
        props[f[1]] = f
    return values, props


def main():
    ucd = sys.argv[1]
    values, props = aliases(ucd)
    tables = []  # encoded strings, by id

    # General_Category
    gc = {}
    for lo, hi, f in records(os.path.join(ucd, "DerivedGeneralCategory.txt")):
        gc.setdefault(f[0], []).append((lo, hi))
    for g, members in GC_GROUPS.items():
        gc[g] = [r for m in members for r in gc[m]]
    gc_names = {}
    for row in values["gc"]:
        short = row[0]
        if short not in gc:
            continue
        tid = len(tables)
        tables.append(encode(gc[short]))
        for n in row:
            gc_names[n] = tid

    # Script and Script_Extensions
    sc = {}
    for lo, hi, f in records(os.path.join(ucd, "Scripts.txt")):
        sc.setdefault(f[0], []).append((lo, hi))
    long_of = {}
    for row in values["sc"]:
        long_of[row[0]] = row[1]
    sc.setdefault("Unknown", [])
    sc["Unknown"] = complement([r for v in sc.values() for r in v])
    scx_explicit = {}
    explicit_ranges = []
    for lo, hi, f in records(os.path.join(ucd, "ScriptExtensions.txt")):
        explicit_ranges.append((lo, hi))
        for short in f[0].split():
            scx_explicit.setdefault(long_of[short], []).append((lo, hi))
    explicit = merge(explicit_ranges)

    def minus(a, b):
        out = []
        for lo, hi in merge(a):
            cur = lo
            for blo, bhi in b:
                if bhi < cur or blo > hi:
                    continue
                if blo > cur:
                    out.append((cur, blo - 1))
                cur = max(cur, bhi + 1)
            if cur <= hi:
                out.append((cur, hi))
        return out

    sc_names, scx_names = {}, {}
    for row in values["sc"]:
        name = row[1]
        if name not in sc:
            continue
        tid = len(tables)
        tables.append(encode(sc[name]))
        xid = len(tables)
        tables.append(encode(minus(sc[name], explicit) + scx_explicit.get(name, [])))
        for n in row:
            sc_names[n] = tid
            scx_names[n] = xid

    # Binary properties
    binary = {name: [] for name in BINARY}
    for fname in ["PropList.txt", "DerivedCoreProperties.txt", "emoji-data.txt",
                  "DerivedBinaryProperties.txt", "DerivedNormalizationProps.txt"]:
        for lo, hi, f in records(os.path.join(ucd, fname)):
            if f[0] in binary and len(f) == 1:
                binary[f[0]].append((lo, hi))
    binary["Any"] = [(0, MAX)]
    binary["ASCII"] = [(0, 0x7F)]
    binary["Assigned"] = complement(gc["Cn"])
    bin_names = {}
    for name in BINARY:
        tid = len(tables)
        tables.append(encode(binary[name]))
        names = props.get(name, [None, name])
        for n in set([name] + names[:2] + names[2:]):
            if n:
                bin_names[n] = tid

    def lookup(names):
        return "|" + "|".join("%s:%d" % (n, i) for n, i in sorted(names.items())) + "|"

    out = []
    out.append("module js\n")
    version = re.search(r"Scripts-([\d.]+)\.txt", open(os.path.join(ucd, "Scripts.txt")).readline()).group(1)
    out.append("// Generated by tools/gen_unicode_props.py from the Unicode %s Character" % version)
    out.append("// Database. Don't edit by hand.\n")
    out.append("/// `|name:id|` for every General_Category value name and alias.")
    out.append('func unicodeGcNames() -> String { "%s" }\n' % lookup(gc_names))
    out.append("/// Script value names and aliases to their Script tables.")
    out.append('func unicodeScriptNames() -> String { "%s" }\n' % lookup(sc_names))
    out.append("/// Script value names and aliases to their Script_Extensions tables.")
    out.append('func unicodeScriptExtensionNames() -> String { "%s" }\n' % lookup(scx_names))
    out.append("/// Binary property names and aliases.")
    out.append('func unicodeBinaryNames() -> String { "%s" }\n' % lookup(bin_names))
    out.append("/// Table `id`: base-36 numbers alternating the gap before a range")
    out.append("/// (from the previous range's end) and its length less one.")
    out.append("func unicodeTable(id: Int64) -> String {")
    out.append("    match id {")
    for i, t in enumerate(tables):
        out.append('        %d => "%s",' % (i, t))
    out.append('        _ => ""')
    out.append("    }")
    out.append("}")
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "unicode_props.ks")
    open(path, "w").write("\n".join(out) + "\n")
    print("%d tables, %d bytes" % (len(tables), sum(len(t) for t in tables)))


main()
