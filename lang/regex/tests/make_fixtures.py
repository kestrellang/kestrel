#!/usr/bin/env python3
"""Writes fixtures.ks: every match (with capture groups) Python's `re`
finds for a table of patterns and texts, iterated the way this package
does (an empty match right after the previous match is skipped, as in
RE2 and Go).

Patterns are written in this package's syntax and translated for Python:
`(?<name>` becomes `(?P<name>` and `\\z` becomes `\\Z`. Everything runs
with re.ASCII, matching this package's ASCII `\\d \\w \\s \\b`.

Usage: python3 lang/regex/tests/make_fixtures.py
"""
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))

CASES = [
    # Literals and concatenation
    ("abc", ["abc", "xabcx", "ab", "abcabc", ""]),
    ("a", ["", "a", "bab", "aaa"]),
    ("hello world", ["say hello world!", "hello  world"]),
    # Dot
    ("a.c", ["abc", "a\nc", "ac", "a.c axc"]),
    ("(?s)a.c", ["a\nc", "abc"]),
    (".", ["", "x", "héllo"]),
    ("...", ["héllo wörld"]),
    # Classes
    ("[abc]+", ["xxabcbay", "zzz"]),
    ("[^abc]+", ["abcxyzabc", "abc"]),
    ("[a-z0-9_]+", ["Hello_World 42!"]),
    ("[-a]+", ["a-a-b"]),
    ("[a-]+", ["--a b"]),
    ("[]a]+", ["]a]b"]),
    ("[^]a]+", ["]a]bcd"]),
    ("[\\d\\s]+", ["ab 12 3c"]),
    ("[^\\d]+", ["12ab34"]),
    ("[\\w.-]+@[\\w.-]+", ["mail me at a.b-c@d.example now"]),
    ("[\\]\\[]+", ["a[]]b"]),
    ("[à-ÿ]+", ["café crème"]),
    # Perl classes
    ("\\d+", ["abc 123 de 45", "none"]),
    ("\\D+", ["12ab34cd"]),
    ("\\w+", ["hello, world_1 2"]),
    ("\\W+", ["a, b; c"]),
    ("\\s+", ["a  b\t\nc"]),
    ("\\S+", [" ab  cd "]),
    # Anchors
    ("^abc", ["abc", "xabc", "abc\nabc"]),
    ("abc$", ["abc", "abcx", "xabc"]),
    ("(?m)^\\w+", ["one\ntwo\nthree"]),
    ("(?m)\\w+$", ["one\ntwo\nthree"]),
    ("(?m)^$", ["a\n\nb", ""]),
    ("^", ["abc", ""]),
    ("$", ["abc", ""]),
    ("\\Aab", ["abab"]),
    ("ab\\z", ["abab", "abab\n"]),
    ("\\bcat\\b", ["cat concat cat. bobcat cats"]),
    ("\\Bcat\\B", ["concatenate cat"]),
    ("\\b", ["ab cd", ""]),
    ("\\B", ["ab cd"]),
    # Alternation
    ("cat|dog", ["hotdog catalog", "bird"]),
    ("a|ab", ["ab"]),
    ("ab|a", ["ab"]),
    ("a|b|c|", ["xabcz"]),
    ("(a|b)(c|d)", ["ac bd ad cb"]),
    ("foo(bar|baz)?", ["foo foobar foobaz foobat"]),
    # Repetition
    ("a*", ["", "aaa", "baaa", "bab"]),
    ("a+", ["baaab aab"]),
    ("a?", ["aab"]),
    ("a{3}", ["aaaaaaa"]),
    ("a{2,}", ["a aa aaaaa"]),
    ("a{2,3}", ["aaaaaaaa"]),
    ("a{0}", ["aaa"]),
    ("a{1,1}b", ["aab"]),
    ("(ab){2}", ["abababab"]),
    ("x{0,2}y", ["xxxy xy y"]),
    ("a{,3}", ["a{,3}"]),
    ("a{x}", ["a{x}"]),
    ("{", ["a{b"]),
    ("a{2", ["aa{2"]),
    # Lazy repetition
    ("a+?", ["aaa"]),
    ("a*?", ["aaa"]),
    ("a??b", ["ab b"]),
    ("<.+>", ["<a> <b>"]),
    ("<.+?>", ["<a> <b>"]),
    ("a{2,4}?", ["aaaaa"]),
    ("(a+?)(a*)", ["aaaa"]),
    # Groups and captures
    ("(a)(b)?", ["ab a"]),
    ("(a)|(b)", ["ab"]),
    ("((a)(b(c)))", ["abc"]),
    ("(?:ab)+", ["ababab"]),
    ("(?:a|(b))+", ["aba"]),
    ("(a*)+", ["b"]),
    ("(a|)+b", ["aab"]),
    ("(?<year>\\d{4})-(?<month>\\d\\d)", ["on 2026-11 and 1999-01"]),
    ("(\\w+)@(\\w+)\\.com", ["bob@example.com, eve@corp.com"]),
    ("(a*)*", ["aa"]),
    ("(a*)+$", ["aab"]),
    ("(a?)*?b", ["aab"]),
    # Case-insensitive
    ("(?i)hello", ["HeLLo hello HELLO"]),
    ("(?i)[a-c]+", ["xABcaBz"]),
    ("(?i:ab)c", ["ABc ABC abc"]),
    ("a(?i)b", ["aB AB ab"]),
    ("(?i)[^a]", ["aAb"]),
    ("(?-i)a", ["Aa"]),
    # Escapes
    ("\\.\\*\\+\\?", ["a.*+?b"]),
    ("\\(\\)", ["f()"]),
    ("\\x41\\x{42}", ["xABy"]),
    ("\\t\\n", ["a\t\nb"]),
    ("\\$\\^", ["$^"]),
    # Unicode
    ("é+", ["cafééé"]),
    ("[é-ë]", ["aêb"]),
    ("😀+", ["hi 😀😀!"]),
    (".😀.", ["a😀b"]),
    # Word-ish patterns
    ("\\b\\w+\\b", ["The quick, brown fox."]),
    ("\\d{3}-\\d{4}", ["call 555-1234 or 555-98765"]),
    ("[A-Z][a-z]+", ["Alice and Bob met Carol"]),
    ("^\\s*#.*$", ["  # comment"]),
    ("(?m)^(\\w+):\\s*(.*)$", ["host: example.com\nport: 8080\n"]),
    ("https?://[^\\s/]+(/\\S*)?", ["see http://a.com and https://b.org/x?y=1."]),
    # Mildly backtracking-heavy (Python takes exponential time on the
    # long versions; regex.ks checks those separately).
    ("(a*)*b", ["a" * 12 + "b"]),
    ("(a|aa)+c", ["a" * 12 + "c"]),
]


# Python spells these differently (no mid-pattern flags, no \x{...}).
PYTHON_SPELLING = {
    "a(?i)b": "a(?i:b)",
    "(?-i)a": "(?-i:a)",
    "\\x41\\x{42}": "\\x41\\x42",
}


# Where Python's backtracking differs from automaton engines: a group in a
# repetition that also matches empty. Python reports a final empty
# iteration; this package, like RE2, reports the last non-empty one.
EXPECTED_OVERRIDES = {
    ("(a|)+b", "aab"): "0-3,1-2",
    ("(a*)*", "aa"): "0-2,0-2",
    ("(a*)*b", "a" * 12 + "b"): "0-13,0-12",
}


def python_pattern(pattern):
    if pattern in PYTHON_SPELLING:
        return PYTHON_SPELLING[pattern]
    return pattern.replace("(?<", "(?P<").replace("\\z", "\\Z")


def all_matches(pattern, text):
    rx = re.compile(python_pattern(pattern), re.ASCII)
    out = []
    pos = 0
    previous_end = -1
    while pos <= len(text):
        m = rx.search(text, pos)
        if not m:
            break
        s, e = m.span()
        if s == e and s == previous_end:
            if s >= len(text):
                break
            pos = s + 1
            continue
        out.append(m)
        previous_end = e
        pos = e if e > s else e + 1
    return out


def byte_offset(text, index):
    return len(text[:index].encode("utf-8"))


def encode_matches(text, matches):
    parts = []
    for m in matches:
        spans = []
        for g in range(len(m.groups()) + 1):
            s, e = m.span(g)
            if s < 0:
                spans.append("x")
            else:
                spans.append(f"{byte_offset(text, s)}-{byte_offset(text, e)}")
        parts.append(",".join(spans))
    return " ".join(parts)


def kestrel_string(text):
    out = []
    for ch in text:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\t":
            out.append("\\t")
        elif ord(ch) < 32 or ord(ch) > 126:
            out.append(f"\\u{{{ord(ch):X}}}")
        else:
            out.append(ch)
    return '"' + "".join(out) + '"'


def main():
    rows = []
    for pattern, texts in CASES:
        for text in texts:
            expected = encode_matches(text, all_matches(pattern, text))
            expected = EXPECTED_OVERRIDES.get((pattern, text), expected)
            rows.append(f"        Case(pattern: {kestrel_string(pattern)}, text: {kestrel_string(text)}, expected: {kestrel_string(expected)})")
    lines = [
        "// Generated by make_fixtures.py from Python's re; do not edit.",
        "",
        "module regex_tests",
        "",
        "struct Case {",
        "    let pattern: String;",
        "    let text: String;",
        '    /// Each match as "start-end" byte spans for group 0, 1, ...',
        '    /// ("x" when a group took no part), matches separated by spaces.',
        "    let expected: String;",
        "}",
        "",
        "func cases() -> Array[Case] {",
        "    [",
        ",\n".join(rows),
        "    ]",
        "}",
        "",
    ]
    with open(os.path.join(HERE, "fixtures.ks"), "w") as f:
        f.write("\n".join(lines))
    print(f"{len(rows)} cases")


if __name__ == "__main__":
    main()
