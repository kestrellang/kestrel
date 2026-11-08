# Regex

Regular expressions for Kestrel, in pure Kestrel. Matching runs in time linear in the text, whatever the pattern, so untrusted patterns and inputs can't hang a program.

## Installation

```toml
[dependencies]
regex = { path = "../../lang/regex" }
```

## Usage

```kestrel
import regex.(Regex)

let word = try Regex.compile("\\b\\w+\\b");      // Result[Regex, RegexError]
word.isMatch(in: "hello");                        // true
for m in word.matches(in: "two words") { print(m.text); }

let pair = try Regex.compile("(?<key>\\w+)=(\\d+)");
let c = pair.captures(in: "x=42")?;
c.named("key")?.text;                             // "x"
c.group(2)?.text;                                 // "42"

let swap = try Regex.compile("(\\w+) (\\w+)");
swap.replacingAll(in: "hello world", with: "$2 $1");          // "world hello"
swap.replacingAll(in: "a b", using: { (c) in c.whole.text.uppercased() });

try Regex.compile(",\\s*").split("a, b,c");      // ["a", "b", "c"]
Regex.escape("1+1=2");                            // "1\\+1=2"
```

## Syntax

| | |
|---|---|
| Literals | Any character; `\n \t \r \f \v \a \0 \xHH \x{HHHH}`; `\` before punctuation |
| Any | `.` (not newline unless `(?s)`) |
| Classes | `[abc]`, `[a-z]`, `[^0-9]`, `[[:alpha:]]`, and `\d \w \s \D \W \S` inside or outside |
| Anchors | `^ $` (line-aware with `(?m)`), `\A \z`, `\b \B` |
| Groups | `(…)`, `(?:…)`, `(?<name>…)`, `(?P<name>…)` |
| Alternation | `a|b` |
| Repetition | `* + ? {n} {n,} {n,m} {,m}`, lazy with a trailing `?`; counts up to 1000 |
| Flags | `(?i)` case-insensitive (Unicode simple folding), `(?m)` multi-line, `(?s)` dot matches newline; scoped as `(?i:…)`, turned off as `(?-i)` |

`\d`, `\w`, `\s` and `\b` are ASCII. There are no backreferences or lookaround, since those rule out linear-time matching.

## Semantics

- Leftmost-first, like Perl, Python and Rust's `regex`: the first alternative that matches wins, and greedy repeats take as much as they can.
- Offsets in `Match.start` and `Match.end` are byte offsets into the UTF-8 text. `.` and classes match whole code points.
- `matches`, `replacingAll` and `split` skip an empty match that touches the end of the previous match, as Rust and Go do.
- In replacement templates, `$1` or `${1}` is a group, `$name` or `${name}` a named group, and `$$` a dollar sign. Unknown groups expand to nothing.
- A group inside a loop reports its last iteration. When that iteration was empty (as in `(a*)+` on `"b"`), the result follows RE2, which can differ from Python.

## Performance

Matching uses a Pike VM: every possible match runs in lockstep, so time is proportional to text length times pattern size. A pattern that starts with an ASCII literal skips ahead to that byte, and a pattern anchored with `^` or `\A` stops after the first position.

On 1.26 MB of text, built with `-O 1`: `\w+@\w+\.com` takes about 0.45 s, the literal `consectetur` about 0.1 s, and `zzz` (no match) about 0.01 s. Python's `re` takes 0.04 s, 0.001 s and 0.001 s. A literal prefilter (memchr-style search for required substrings) and a lazy DFA are the obvious next steps.

## Tests

```sh
lang/regex/tests/run.sh
```

Runs 130 cases generated from Python's `re` (`tests/make_fixtures.py` regenerates `fixtures.ks`), plus API, Unicode case-folding, error-position and linear-time tests (patterns like `(a*)*b` on long input that backtracking engines can't finish).
