module js

// Character classes the lexer needs: identifier characters (UAX #31, via
// tables made by tools/generate_unicode.py), whitespace and line breaks.

/// ID_Start as sorted (lo, hi) code point pairs.
@fileconstant("data/id_start.bin")
let ID_START: LiteralSlice[Int32]

/// ID_Continue as sorted (lo, hi) code point pairs.
@fileconstant("data/id_continue.bin")
let ID_CONTINUE: LiteralSlice[Int32]

/// Binary search for `c` in a table of (lo, hi) pairs.
func inRanges(table: LiteralSlice[Int32], c: Int64) -> Bool {
    var lo: Int64 = 0;
    var hi = table.count / 2;
    while lo < hi {
        let mid = (lo + hi) / 2;
        let start = Int64(from: unsafe table(unchecked: mid * 2));
        let end = Int64(from: unsafe table(unchecked: mid * 2 + 1));
        if c < start {
            hi = mid;
        } else if c > end {
            lo = mid + 1;
        } else {
            return true;
        }
    }
    false
}

/// Can start an identifier: `$`, `_`, or ID_Start.
func isIdStart(c: Int64) -> Bool {
    if c < 128 {
        return (c >= 97 and c <= 122) or (c >= 65 and c <= 90) or c == 36 or c == 95;
    }
    inRanges(ID_START, c)
}

/// Can continue an identifier: `$`, ID_Continue, ZWNJ or ZWJ.
func isIdPart(c: Int64) -> Bool {
    if c < 128 {
        return (c >= 97 and c <= 122) or (c >= 65 and c <= 90) or (c >= 48 and c <= 57) or c == 36 or c == 95;
    }
    c == 0x200C or c == 0x200D or inRanges(ID_CONTINUE, c)
}

/// LF, CR, LINE SEPARATOR, PARAGRAPH SEPARATOR.
func isLineTerminator(c: Int64) -> Bool {
    c == 10 or c == 13 or c == 0x2028 or c == 0x2029
}

/// Tab, vertical tab, form feed, space, no-break space, BOM, and the
/// other Space_Separator characters.
func isWhitespace(c: Int64) -> Bool {
    if c == 32 or c == 9 or c == 11 or c == 12 { return true; }
    if c < 128 { return false; }
    c == 0xA0 or c == 0xFEFF or c == 0x1680 or (c >= 0x2000 and c <= 0x200A)
        or c == 0x202F or c == 0x205F or c == 0x3000
}

func isDecimalDigit(c: Int64) -> Bool { c >= 48 and c <= 57 }

/// The value of hex digit `c`, or -1.
func hexValue(c: Int64) -> Int64 {
    if c >= 48 and c <= 57 { return c - 48; }
    if c >= 97 and c <= 102 { return c - 87; }
    if c >= 65 and c <= 70 { return c - 55; }
    return -1;
}

/// Appends code point `c` to `out` as UTF-8. Lone surrogates, which JS
/// strings can hold but UTF-8 can't, become U+FFFD.
/// Appends code point `c`. A lone surrogate (from a `\uD83D` escape) is
/// kept, WTF-8 encoded, so `unitsOf` turns it back into one code unit.
func appendCodePoint(mutating out: String, c: Int64) {
    let valid = c >= 0 and c <= 0x10FFFF;
    let scalar = if valid { UInt32(truncating: c) } else { UInt32(truncating: 0xFFFD) };
    out.append(char: Char(unchecked: scalar));
}
