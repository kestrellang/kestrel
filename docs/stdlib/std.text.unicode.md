# std.text.unicode

## enum `GraphemeBreakProperty`

```kestrel
public enum GraphemeBreakProperty
```

One of the UAX #29 Grapheme_Cluster_Break property values.

Returned by `graphemeBreakProperty(c:)` and consumed by
`shouldBreakBetween(...)`. Variant names match the Unicode property
labels — see UAX #29 for the precise definitions and the boundary
rules (GB1–GB999) that consume them.

### Representation

A 14-state tag enum (no payload). `ordinal()` gives the numeric
encoding used by the stage-2 lookup table.

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

### Members

#### case `CR`

```kestrel
case CR
```

Carriage Return (`U+000D`).

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `Control`

```kestrel
case Control
```

Other control characters that always force a break.

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `Extend`

```kestrel
case Extend
```

Combining marks and other extending characters.

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `L`

```kestrel
case L
```

Hangul leading consonant jamo (L).

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `LF`

```kestrel
case LF
```

Line Feed (`U+000A`).

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `LV`

```kestrel
case LV
```

Hangul precomposed LV syllable.

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `LVT`

```kestrel
case LVT
```

Hangul precomposed LVT syllable.

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `Other`

```kestrel
case Other
```

Default class — anything not in another category.

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `Prepend`

```kestrel
case Prepend
```

Prepended concatenation marks.

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `RegionalIndicator`

```kestrel
case RegionalIndicator
```

Regional Indicator codepoints (used to form flag emoji in pairs).

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `SpacingMark`

```kestrel
case SpacingMark
```

Spacing combining marks.

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `T`

```kestrel
case T
```

Hangul trailing consonant jamo (T).

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `V`

```kestrel
case V
```

Hangul vowel jamo (V).

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### case `ZWJ`

```kestrel
case ZWJ
```

Zero Width Joiner (`U+200D`).

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

#### function `ordinal`

```kestrel
func ordinal() -> Int32
```

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: GraphemeBreakProperty) -> Bool
```

Equality by ordinal — same variant, same value.

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

### Implements `Matchable`

#### function `matches`

```kestrel
public func matches(GraphemeBreakProperty) -> Bool
```

Match form of `isEqual` for use in pattern matching.

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

## function `caseFold`

```kestrel
public func caseFold(Char) -> Char
```

Single-codepoint case fold for `c`. Use when comparing characters
case-insensitively: `caseFold(a) == caseFold(b)` is the canonical
per-character equality test.

For codepoints whose Unicode fold is multi-codepoint (e.g. `ß → ss`),
this returns only the first folded codepoint — see
`caseFoldExpansion` for the full sequence.

### Examples

```
caseFold('A')                       // 'a'
caseFold('İ')                       // 'i' — see caseFoldExpansion for "i\u{307}"
caseFold('a') == caseFold('A')      // true
```

_Defined in `lang/std/text/unicode/case_folding.ks`._

## function `caseFoldExpansion`

```kestrel
public func caseFoldExpansion(Char) -> String
```

Full Unicode case fold of `c` as a `String`. Returns `""` when
`c` has no multi-codepoint fold — pair with `hasCaseFoldExpansion`,
or fall back to `caseFold` for the single-codepoint form.

### Examples

```
caseFoldExpansion('ß')              // "ss"
caseFoldExpansion('ﬃ')             // "ffi"
caseFoldExpansion('a')              // ""  (no expansion; caseFold('a') == 'a')
```

_Defined in `lang/std/text/unicode/case_folding.ks`._

## function `graphemeBreakProperty`

```kestrel
public func graphemeBreakProperty(Char) -> GraphemeBreakProperty
```

Looks up the UAX #29 Grapheme_Cluster_Break property for `c`.

O(1) — two array indexings into the trie. Codepoints above
`U+10FFFF` (which are not valid Unicode scalars) yield `.Other`.

### Examples

```
graphemeBreakProperty('a')                  // .Other
graphemeBreakProperty('\r')                 // .CR
graphemeBreakProperty('\u{200D}')           // .ZWJ
```

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

## function `hasCaseFoldExpansion`

```kestrel
public func hasCaseFoldExpansion(Char) -> Bool
```

`true` iff folding `c` produces more than one codepoint. Linear
scan over `FOLD_EXPANSIONS` (~100 entries) — fine per-character but
quadratic if applied across a large set.

_Defined in `lang/std/text/unicode/case_folding.ks`._

## function `hasLowercaseExpansion`

```kestrel
public func hasLowercaseExpansion(Char) -> Bool
```

`true` iff lowercasing `c` produces more than one codepoint.
Same scan caveats as `hasUppercaseExpansion`.

_Defined in `lang/std/text/unicode/case_tables.ks`._

## function `hasTitlecaseExpansion`

```kestrel
public func hasTitlecaseExpansion(Char) -> Bool
```

`true` iff titlecasing `c` produces more than one codepoint.

_Defined in `lang/std/text/unicode/case_tables.ks`._

## function `hasUppercaseExpansion`

```kestrel
public func hasUppercaseExpansion(Char) -> Bool
```

`true` iff uppercasing `c` produces more than one codepoint.
Linear scan over `UPPER_EXPANSIONS` (~100 entries); fine for
per-character calls in normal text but quadratic if applied to a
large codepoint set.

_Defined in `lang/std/text/unicode/case_tables.ks`._

## function `lowercaseExpansion`

```kestrel
public func lowercaseExpansion(Char) -> String
```

Full Unicode lowercase expansion for `c`. Empty string when no
multi-codepoint expansion applies — see `uppercaseExpansion` for
the same shape.

_Defined in `lang/std/text/unicode/case_tables.ks`._

## function `shouldBreakBetween`

```kestrel
public func shouldBreakBetween(GraphemeBreakProperty, GraphemeBreakProperty, Bool, Bool) -> Bool
```

Decides whether a grapheme cluster boundary lies between two
adjacent codepoints with the given break properties.

Implements the UAX #29 boundary rules GB3–GB13/GB999. The
caller must thread two scalar bits across calls to capture rules
that look further back than one codepoint:

- `prevPrevWasRI`: was the codepoint *before* `prev` a
  Regional_Indicator? Needed to keep regional-indicator pairs
  together while still breaking between successive pairs (GB12/13).
- `prevWasZWJ`: was the codepoint before `prev` a ZWJ? Needed for
  the simplified emoji ZWJ-sequence rule (GB11).

Returns `true` to break (start a new cluster at `curr`), `false` to
keep `prev` and `curr` in the same cluster.

_Defined in `lang/std/text/unicode/grapheme_tables.ks`._

## function `titlecaseExpansion`

```kestrel
public func titlecaseExpansion(Char) -> String
```

Full Unicode titlecase expansion for `c`. Empty string when no
multi-codepoint expansion applies.

_Defined in `lang/std/text/unicode/case_tables.ks`._

## function `toLowercase`

```kestrel
public func toLowercase(Char) -> Char
```

Single-codepoint lowercase mapping for `c`. Same caveats as
`toUppercase`: codepoints with multi-char lowercase forms return
only the first codepoint — see `lowercaseExpansion`.

_Defined in `lang/std/text/unicode/case_tables.ks`._

## function `toTitlecase`

```kestrel
public func toTitlecase(Char) -> Char
```

Single-codepoint titlecase mapping for `c`. Differs from
`toUppercase` only for the codepoints (mostly Greek/Croatian
digraphs) where Unicode defines a distinct "Title" form. Multi-char
expansions live in `titlecaseExpansion`.

_Defined in `lang/std/text/unicode/case_tables.ks`._

## function `toUppercase`

```kestrel
public func toUppercase(Char) -> Char
```

Single-codepoint uppercase mapping for `c`. Falls back to `c` for
characters with no mapping and for codepoints above `U+10FFFF`.

For characters whose Unicode uppercase form expands to multiple
codepoints (e.g. `ß → SS`, `ﬁ → FI`), this returns only the first
codepoint of the expansion. Use `hasUppercaseExpansion(c:)` to detect
the multi-char case and `uppercaseExpansion(c:)` to retrieve the full
`String`.

### Examples

```
toUppercase('a')           // 'A'
toUppercase('ß')           // 'S' — see uppercaseExpansion for "SS"
toUppercase('1')           // '1' — no mapping
```

_Defined in `lang/std/text/unicode/case_tables.ks`._

## field `unicodeVersion`

```kestrel
public let unicodeVersion: String
```

Unicode version these tables track. Bump alongside the regeneration
of the underlying `data/*.bin` files.

_Defined in `lang/std/text/unicode/case_tables.ks`._

## function `uppercaseExpansion`

```kestrel
public func uppercaseExpansion(Char) -> String
```

Full Unicode uppercase expansion for `c` as a `String`. Returns the
empty string when `c` has no multi-codepoint expansion — pair with
`hasUppercaseExpansion` (or call `toUppercase` instead) to avoid the
scan when you only need the single-codepoint form.

### Examples

```
uppercaseExpansion('ß')          // "SS"
uppercaseExpansion('ﬁ')          // "FI"
uppercaseExpansion('a')          // ""  (use toUppercase for 'A')
```

_Defined in `lang/std/text/unicode/case_tables.ks`._

