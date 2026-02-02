# Unicode Support

Kestrel provides full Unicode support through embedded lookup tables generated from the Unicode Character Database (UCD).

## Data Sources

All tables are generated from the official Unicode Consortium data files:

| File | URL | Used For |
|------|-----|----------|
| `UnicodeData.txt` | https://www.unicode.org/Public/15.1.0/ucd/UnicodeData.txt | Case mappings, general category |
| `SpecialCasing.txt` | https://www.unicode.org/Public/15.1.0/ucd/SpecialCasing.txt | Context-sensitive and 1-to-many case mappings |
| `CaseFolding.txt` | https://www.unicode.org/Public/15.1.0/ucd/CaseFolding.txt | Case-insensitive comparison |
| `GraphemeBreakProperty.txt` | https://www.unicode.org/Public/15.1.0/ucd/auxiliary/GraphemeBreakProperty.txt | Grapheme cluster boundaries (UAX #29) |

## Table Generation

A build script downloads the UCD files and generates Kestrel source:

```bash
# Generate unicode tables
python3 scripts/generate_unicode_tables.py

# Output: std/text/unicode/tables.ks
```

The generator produces:
- `case_tables.ks` - Uppercase/lowercase/titlecase mappings
- `case_folding.ks` - Case folding for case-insensitive comparison
- `grapheme_tables.ks` - Grapheme cluster break properties

## Table Format

### Two-Stage Lookup

Tables use a two-stage lookup to compress sparse Unicode data:

```
Stage 1: codePoint >> 8  →  block index
Stage 2: block[codePoint & 0xFF]  →  property value
```

This exploits the fact that Unicode properties are often constant within 256-codepoint blocks.

### Case Mapping

Most case conversions are a simple delta (e.g., `A` + 32 = `a`). Special cases (1-to-many mappings) are stored separately:

```kestrel
// Simple case: delta stored directly
// 'A' (0x41) -> 'a' (0x61) = delta of +32

// Special case: ß -> SS (1 char becomes 2)
// Stored in SPECIAL_CASES array with explicit mapping
```

Notable special cases:
- `ß` (U+00DF) → `SS` (uppercase)
- `ﬁ` (U+FB01) → `FI` (uppercase)
- `ΐ` (U+0390) → `Ι` + combining marks

### Grapheme Clusters

Grapheme break properties follow UAX #29. Each codepoint maps to a break property:

```kestrel
enum GraphemeBreakProperty {
    case Other
    case CR
    case LF
    case Control
    case Extend
    case ZWJ
    case RegionalIndicator
    case Prepend
    case SpacingMark
    case L   // Hangul syllables
    case V
    case T
    case LV
    case LVT
}
```

The segmentation algorithm uses these properties to determine cluster boundaries.

## Size Budget

| Component | Size |
|-----------|-----:|
| Case mapping (simple) | ~45 KB |
| Case mapping (special) | ~8 KB |
| Case folding | ~12 KB |
| Grapheme break | ~25 KB |
| **Total** | **~90 KB** |

Comparable to other languages:
- Rust std: ~100 KB
- Go: ~80 KB
- Swift: Uses system ICU

## Updating Unicode Version

To update to a new Unicode version:

1. Update URLs in `scripts/generate_unicode_tables.py`
2. Run the generator
3. Run tests to catch any breaking changes
4. Update version constant in `std/text/unicode.ks`

```kestrel
/// The Unicode version these tables were generated from.
public let unicodeVersion: String = "15.1.0";
```

## API

Once tables are embedded, full Unicode methods become available:

```kestrel
// Full Unicode case conversion
let s = "MÜNCHEN";
s.lowercased();  // "münchen"

// Case-insensitive comparison
"straße".equalsCaseInsensitive("STRASSE");  // true

// Proper grapheme iteration
let emoji = "👨‍👩‍👧";
emoji.graphemes.count();  // 1 (not 5)
```

## Future Work

- **Locale-aware casing**: Turkish `i` ↔ `İ` (not `I`)
- **Word/sentence boundaries**: UAX #29 word and sentence break properties
- **Normalization**: NFC, NFD, NFKC, NFKD (UAX #15)
- **Collation**: Locale-aware sorting (UCA)
