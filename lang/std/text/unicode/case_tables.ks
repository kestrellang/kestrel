// Unicode case mapping tables
//
// Two-stage trie tables (block index + delta) loaded via `@fileconstant`,
// plus a small literal expansion array for the codepoints that do not
// case-map 1:1. Generated from the Unicode data files by
// `generate_data.py`; do not edit the `data/*.bin` files by hand.

module std.text.unicode

import std.text.(Char, String)
import std.numeric.(Int64, Int32, UInt32)
import std.core.(Bool)
import std.memory.(LiteralSlice)

/// Unicode version these tables track. Bump alongside the regeneration
/// of the underlying `data/*.bin` files.
public let unicodeVersion: String = "15.1.0"

// ============================================================================
// TWO-STAGE LOOKUP TABLES (loaded from binary files)
// ============================================================================

/// UPPER case mapping stage 1 table (block indices)
@fileconstant("data/upper_stage1.bin")
let UPPER_STAGE1: LiteralSlice[Int32]

/// UPPER case mapping stage 2 table (delta values)
@fileconstant("data/upper_stage2.bin")
let UPPER_STAGE2: LiteralSlice[Int32]

/// LOWER case mapping stage 1 table (block indices)
@fileconstant("data/lower_stage1.bin")
let LOWER_STAGE1: LiteralSlice[Int32]

/// LOWER case mapping stage 2 table (delta values)
@fileconstant("data/lower_stage2.bin")
let LOWER_STAGE2: LiteralSlice[Int32]

/// TITLE case mapping stage 1 table (block indices)
@fileconstant("data/title_stage1.bin")
let TITLE_STAGE1: LiteralSlice[Int32]

/// TITLE case mapping stage 2 table (delta values)
@fileconstant("data/title_stage2.bin")
let TITLE_STAGE2: LiteralSlice[Int32]

// ============================================================================
// EXPANSION ARRAYS (kept as literals - small data)
// ============================================================================

/// Uppercase expansions: (codepoint, expansion_length, char1, char2, char3)
let UPPER_EXPANSIONS_COUNT: Int64 = 102;
let UPPER_EXPANSIONS: std.collections.Array[(Int32, Int32, Int32, Int32, Int32)] = [
    (223, 2, 83, 83, 0),  // U+00DF
    (64256, 2, 70, 70, 0),  // U+FB00
    (64257, 2, 70, 73, 0),  // U+FB01
    (64258, 2, 70, 76, 0),  // U+FB02
    (64259, 3, 70, 70, 73),  // U+FB03
    (64260, 3, 70, 70, 76),  // U+FB04
    (64261, 2, 83, 84, 0),  // U+FB05
    (64262, 2, 83, 84, 0),  // U+FB06
    (1415, 2, 1333, 1362, 0),  // U+0587
    (64275, 2, 1348, 1350, 0),  // U+FB13
    (64276, 2, 1348, 1333, 0),  // U+FB14
    (64277, 2, 1348, 1339, 0),  // U+FB15
    (64278, 2, 1358, 1350, 0),  // U+FB16
    (64279, 2, 1348, 1341, 0),  // U+FB17
    (329, 2, 700, 78, 0),  // U+0149
    (912, 3, 921, 776, 769),  // U+0390
    (944, 3, 933, 776, 769),  // U+03B0
    (496, 2, 74, 780, 0),  // U+01F0
    (7830, 2, 72, 817, 0),  // U+1E96
    (7831, 2, 84, 776, 0),  // U+1E97
    (7832, 2, 87, 778, 0),  // U+1E98
    (7833, 2, 89, 778, 0),  // U+1E99
    (7834, 2, 65, 702, 0),  // U+1E9A
    (8016, 2, 933, 787, 0),  // U+1F50
    (8018, 3, 933, 787, 768),  // U+1F52
    (8020, 3, 933, 787, 769),  // U+1F54
    (8022, 3, 933, 787, 834),  // U+1F56
    (8118, 2, 913, 834, 0),  // U+1FB6
    (8134, 2, 919, 834, 0),  // U+1FC6
    (8146, 3, 921, 776, 768),  // U+1FD2
    (8147, 3, 921, 776, 769),  // U+1FD3
    (8150, 2, 921, 834, 0),  // U+1FD6
    (8151, 3, 921, 776, 834),  // U+1FD7
    (8162, 3, 933, 776, 768),  // U+1FE2
    (8163, 3, 933, 776, 769),  // U+1FE3
    (8164, 2, 929, 787, 0),  // U+1FE4
    (8166, 2, 933, 834, 0),  // U+1FE6
    (8167, 3, 933, 776, 834),  // U+1FE7
    (8182, 2, 937, 834, 0),  // U+1FF6
    (8064, 2, 7944, 921, 0),  // U+1F80
    (8065, 2, 7945, 921, 0),  // U+1F81
    (8066, 2, 7946, 921, 0),  // U+1F82
    (8067, 2, 7947, 921, 0),  // U+1F83
    (8068, 2, 7948, 921, 0),  // U+1F84
    (8069, 2, 7949, 921, 0),  // U+1F85
    (8070, 2, 7950, 921, 0),  // U+1F86
    (8071, 2, 7951, 921, 0),  // U+1F87
    (8072, 2, 7944, 921, 0),  // U+1F88
    (8073, 2, 7945, 921, 0),  // U+1F89
    (8074, 2, 7946, 921, 0),  // U+1F8A
    (8075, 2, 7947, 921, 0),  // U+1F8B
    (8076, 2, 7948, 921, 0),  // U+1F8C
    (8077, 2, 7949, 921, 0),  // U+1F8D
    (8078, 2, 7950, 921, 0),  // U+1F8E
    (8079, 2, 7951, 921, 0),  // U+1F8F
    (8080, 2, 7976, 921, 0),  // U+1F90
    (8081, 2, 7977, 921, 0),  // U+1F91
    (8082, 2, 7978, 921, 0),  // U+1F92
    (8083, 2, 7979, 921, 0),  // U+1F93
    (8084, 2, 7980, 921, 0),  // U+1F94
    (8085, 2, 7981, 921, 0),  // U+1F95
    (8086, 2, 7982, 921, 0),  // U+1F96
    (8087, 2, 7983, 921, 0),  // U+1F97
    (8088, 2, 7976, 921, 0),  // U+1F98
    (8089, 2, 7977, 921, 0),  // U+1F99
    (8090, 2, 7978, 921, 0),  // U+1F9A
    (8091, 2, 7979, 921, 0),  // U+1F9B
    (8092, 2, 7980, 921, 0),  // U+1F9C
    (8093, 2, 7981, 921, 0),  // U+1F9D
    (8094, 2, 7982, 921, 0),  // U+1F9E
    (8095, 2, 7983, 921, 0),  // U+1F9F
    (8096, 2, 8040, 921, 0),  // U+1FA0
    (8097, 2, 8041, 921, 0),  // U+1FA1
    (8098, 2, 8042, 921, 0),  // U+1FA2
    (8099, 2, 8043, 921, 0),  // U+1FA3
    (8100, 2, 8044, 921, 0),  // U+1FA4
    (8101, 2, 8045, 921, 0),  // U+1FA5
    (8102, 2, 8046, 921, 0),  // U+1FA6
    (8103, 2, 8047, 921, 0),  // U+1FA7
    (8104, 2, 8040, 921, 0),  // U+1FA8
    (8105, 2, 8041, 921, 0),  // U+1FA9
    (8106, 2, 8042, 921, 0),  // U+1FAA
    (8107, 2, 8043, 921, 0),  // U+1FAB
    (8108, 2, 8044, 921, 0),  // U+1FAC
    (8109, 2, 8045, 921, 0),  // U+1FAD
    (8110, 2, 8046, 921, 0),  // U+1FAE
    (8111, 2, 8047, 921, 0),  // U+1FAF
    (8115, 2, 913, 921, 0),  // U+1FB3
    (8124, 2, 913, 921, 0),  // U+1FBC
    (8131, 2, 919, 921, 0),  // U+1FC3
    (8140, 2, 919, 921, 0),  // U+1FCC
    (8179, 2, 937, 921, 0),  // U+1FF3
    (8188, 2, 937, 921, 0),  // U+1FFC
    (8114, 2, 8122, 921, 0),  // U+1FB2
    (8116, 2, 902, 921, 0),  // U+1FB4
    (8130, 2, 8138, 921, 0),  // U+1FC2
    (8132, 2, 905, 921, 0),  // U+1FC4
    (8178, 2, 8186, 921, 0),  // U+1FF2
    (8180, 2, 911, 921, 0),  // U+1FF4
    (8119, 3, 913, 834, 921),  // U+1FB7
    (8135, 3, 919, 834, 921),  // U+1FC7
    (8183, 3, 937, 834, 921),  // U+1FF7
]

/// Lowercase expansions: (codepoint, expansion_length, char1, char2, char3)
let LOWER_EXPANSIONS_COUNT: Int64 = 1;
let LOWER_EXPANSIONS: std.collections.Array[(Int32, Int32, Int32, Int32, Int32)] = [
    (304, 2, 105, 775, 0),  // U+0130
]

/// Titlecase expansions: (codepoint, expansion_length, char1, char2, char3)
let TITLE_EXPANSIONS_COUNT: Int64 = 48;
let TITLE_EXPANSIONS: std.collections.Array[(Int32, Int32, Int32, Int32, Int32)] = [
    (223, 2, 83, 115, 0),  // U+00DF
    (64256, 2, 70, 102, 0),  // U+FB00
    (64257, 2, 70, 105, 0),  // U+FB01
    (64258, 2, 70, 108, 0),  // U+FB02
    (64259, 3, 70, 102, 105),  // U+FB03
    (64260, 3, 70, 102, 108),  // U+FB04
    (64261, 2, 83, 116, 0),  // U+FB05
    (64262, 2, 83, 116, 0),  // U+FB06
    (1415, 2, 1333, 1410, 0),  // U+0587
    (64275, 2, 1348, 1398, 0),  // U+FB13
    (64276, 2, 1348, 1381, 0),  // U+FB14
    (64277, 2, 1348, 1387, 0),  // U+FB15
    (64278, 2, 1358, 1398, 0),  // U+FB16
    (64279, 2, 1348, 1389, 0),  // U+FB17
    (329, 2, 700, 78, 0),  // U+0149
    (912, 3, 921, 776, 769),  // U+0390
    (944, 3, 933, 776, 769),  // U+03B0
    (496, 2, 74, 780, 0),  // U+01F0
    (7830, 2, 72, 817, 0),  // U+1E96
    (7831, 2, 84, 776, 0),  // U+1E97
    (7832, 2, 87, 778, 0),  // U+1E98
    (7833, 2, 89, 778, 0),  // U+1E99
    (7834, 2, 65, 702, 0),  // U+1E9A
    (8016, 2, 933, 787, 0),  // U+1F50
    (8018, 3, 933, 787, 768),  // U+1F52
    (8020, 3, 933, 787, 769),  // U+1F54
    (8022, 3, 933, 787, 834),  // U+1F56
    (8118, 2, 913, 834, 0),  // U+1FB6
    (8134, 2, 919, 834, 0),  // U+1FC6
    (8146, 3, 921, 776, 768),  // U+1FD2
    (8147, 3, 921, 776, 769),  // U+1FD3
    (8150, 2, 921, 834, 0),  // U+1FD6
    (8151, 3, 921, 776, 834),  // U+1FD7
    (8162, 3, 933, 776, 768),  // U+1FE2
    (8163, 3, 933, 776, 769),  // U+1FE3
    (8164, 2, 929, 787, 0),  // U+1FE4
    (8166, 2, 933, 834, 0),  // U+1FE6
    (8167, 3, 933, 776, 834),  // U+1FE7
    (8182, 2, 937, 834, 0),  // U+1FF6
    (8114, 2, 8122, 837, 0),  // U+1FB2
    (8116, 2, 902, 837, 0),  // U+1FB4
    (8130, 2, 8138, 837, 0),  // U+1FC2
    (8132, 2, 905, 837, 0),  // U+1FC4
    (8178, 2, 8186, 837, 0),  // U+1FF2
    (8180, 2, 911, 837, 0),  // U+1FF4
    (8119, 3, 913, 834, 837),  // U+1FB7
    (8135, 3, 919, 834, 837),  // U+1FC7
    (8183, 3, 937, 834, 837),  // U+1FF7
]

// ============================================================================
// CASE CONVERSION FUNCTIONS
// ============================================================================

/// Single-codepoint uppercase mapping for `c`. Falls back to `c` for
/// characters with no mapping and for codepoints above `U+10FFFF`.
///
/// For characters whose Unicode uppercase form expands to multiple
/// codepoints (e.g. `ß → SS`, `ﬁ → FI`), this returns only the first
/// codepoint of the expansion. Use `hasUppercaseExpansion(c:)` to detect
/// the multi-char case and `uppercaseExpansion(c:)` to retrieve the full
/// `String`.
///
/// # Examples
///
/// ```
/// toUppercase('a')           // 'A'
/// toUppercase('ß')           // 'S' — see uppercaseExpansion for "SS"
/// toUppercase('1')           // '1' — no mapping
/// ```
public func toUppercase(c: Char) -> Char {
    let cp = c.value();
    // ASCII fast path
    if cp >= 97 and cp <= 122 {
        return Char(cp - 32)
    }
    if cp > 0x10FFFF { return c }
    let blockIdx = UPPER_STAGE1(unchecked: Int64(from: cp.shiftRight(by: 8)));
    let stage2_idx = Int64(from: blockIdx).multiply(256).add(Int64(from: cp.bitwiseAnd(UInt32(intLiteral: 0xFF))));
    let delta = UPPER_STAGE2(unchecked: stage2_idx);
    if delta == 0 { return c }
    Char(UInt32(from: Int64(from: cp).add(Int64(from: delta))))
}

/// Single-codepoint lowercase mapping for `c`. Same caveats as
/// `toUppercase`: codepoints with multi-char lowercase forms return
/// only the first codepoint — see `lowercaseExpansion`.
public func toLowercase(c: Char) -> Char {
    let cp = c.value();
    // ASCII fast path
    if cp >= 65 and cp <= 90 {
        return Char(cp + 32)
    }
    if cp > 0x10FFFF { return c }
    let blockIdx = LOWER_STAGE1(unchecked: Int64(from: cp.shiftRight(by: 8)));
    let stage2_idx = Int64(from: blockIdx).multiply(256).add(Int64(from: cp.bitwiseAnd(UInt32(intLiteral: 0xFF))));
    let delta = LOWER_STAGE2(unchecked: stage2_idx);
    if delta == 0 { return c }
    Char(UInt32(from: Int64(from: cp).add(Int64(from: delta))))
}

/// Single-codepoint titlecase mapping for `c`. Differs from
/// `toUppercase` only for the codepoints (mostly Greek/Croatian
/// digraphs) where Unicode defines a distinct "Title" form. Multi-char
/// expansions live in `titlecaseExpansion`.
public func toTitlecase(c: Char) -> Char {
    let cp = c.value();
    // ASCII fast path (same as uppercase)
    if cp >= 97 and cp <= 122 {
        return Char(cp - 32)
    }
    if cp > 0x10FFFF { return c }
    let blockIdx = TITLE_STAGE1(unchecked: Int64(from: cp.shiftRight(by: 8)));
    let stage2_idx = Int64(from: blockIdx).multiply(256).add(Int64(from: cp.bitwiseAnd(UInt32(intLiteral: 0xFF))));
    let delta = TITLE_STAGE2(unchecked: stage2_idx);
    if delta == 0 { return c }
    Char(UInt32(from: Int64(from: cp).add(Int64(from: delta))))
}

/// `true` iff uppercasing `c` produces more than one codepoint.
/// Linear scan over `UPPER_EXPANSIONS` (~100 entries); fine for
/// per-character calls in normal text but quadratic if applied to a
/// large codepoint set.
public func hasUppercaseExpansion(c: Char) -> Bool {
    let cp = c.value();
    var i: Int64 = 0;
    while i < UPPER_EXPANSIONS_COUNT {
        let entry = UPPER_EXPANSIONS(unchecked: i);
        if UInt32(from: entry.0) == cp { return true }
        i = i + 1
    }
    false
}

/// Full Unicode uppercase expansion for `c` as a `String`. Returns the
/// empty string when `c` has no multi-codepoint expansion — pair with
/// `hasUppercaseExpansion` (or call `toUppercase` instead) to avoid the
/// scan when you only need the single-codepoint form.
///
/// # Examples
///
/// ```
/// uppercaseExpansion('ß')          // "SS"
/// uppercaseExpansion('ﬁ')          // "FI"
/// uppercaseExpansion('a')          // ""  (use toUppercase for 'A')
/// ```
public func uppercaseExpansion(c: Char) -> String {
    let cp = c.value();
    var i: Int64 = 0;
    while i < UPPER_EXPANSIONS_COUNT {
        let entry = UPPER_EXPANSIONS(unchecked: i);
        if UInt32(from: entry.0) == cp {
            var result = String();
            result.appendChar(Char(UInt32(from: entry.2)));
            if entry.1 >= 2 { result.appendChar(Char(UInt32(from: entry.3))) }
            if entry.1 >= 3 { result.appendChar(Char(UInt32(from: entry.4))) }
            return result
        }
        i = i + 1
    }
    ""
}

/// `true` iff lowercasing `c` produces more than one codepoint.
/// Same scan caveats as `hasUppercaseExpansion`.
public func hasLowercaseExpansion(c: Char) -> Bool {
    let cp = c.value();
    var i: Int64 = 0;
    while i < LOWER_EXPANSIONS_COUNT {
        let entry = LOWER_EXPANSIONS(unchecked: i);
        if UInt32(from: entry.0) == cp { return true }
        i = i + 1
    }
    false
}

/// Full Unicode lowercase expansion for `c`. Empty string when no
/// multi-codepoint expansion applies — see `uppercaseExpansion` for
/// the same shape.
public func lowercaseExpansion(c: Char) -> String {
    let cp = c.value();
    var i: Int64 = 0;
    while i < LOWER_EXPANSIONS_COUNT {
        let entry = LOWER_EXPANSIONS(unchecked: i);
        if UInt32(from: entry.0) == cp {
            var result = String();
            result.appendChar(Char(UInt32(from: entry.2)));
            if entry.1 >= 2 { result.appendChar(Char(UInt32(from: entry.3))) }
            if entry.1 >= 3 { result.appendChar(Char(UInt32(from: entry.4))) }
            return result
        }
        i = i + 1
    }
    ""
}

/// `true` iff titlecasing `c` produces more than one codepoint.
public func hasTitlecaseExpansion(c: Char) -> Bool {
    let cp = c.value();
    var i: Int64 = 0;
    while i < TITLE_EXPANSIONS_COUNT {
        let entry = TITLE_EXPANSIONS(unchecked: i);
        if UInt32(from: entry.0) == cp { return true }
        i = i + 1
    }
    false
}

/// Full Unicode titlecase expansion for `c`. Empty string when no
/// multi-codepoint expansion applies.
public func titlecaseExpansion(c: Char) -> String {
    let cp = c.value();
    var i: Int64 = 0;
    while i < TITLE_EXPANSIONS_COUNT {
        let entry = TITLE_EXPANSIONS(unchecked: i);
        if UInt32(from: entry.0) == cp {
            var result = String();
            result.appendChar(Char(UInt32(from: entry.2)));
            if entry.1 >= 2 { result.appendChar(Char(UInt32(from: entry.3))) }
            if entry.1 >= 3 { result.appendChar(Char(UInt32(from: entry.4))) }
            return result
        }
        i = i + 1
    }
    ""
}
