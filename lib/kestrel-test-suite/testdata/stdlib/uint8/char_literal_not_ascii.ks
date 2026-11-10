// test: diagnostics
// stdlib: true

// A character literal typed as `UInt8` is one byte, so it must be ASCII
// (E120). A wider character is several UTF-8 bytes, and truncating it would
// silently keep the wrong one. `Char` still takes any scalar.
module Test

import std.numeric.(UInt8)

func notAscii(b: UInt8) {
    let a: UInt8 = 'é';           // ERROR: character literal 'é' is not ASCII, so it is not one `UInt8` byte
    let e: UInt8 = '\u{1F600}';   // ERROR: character literal '😀' is not ASCII, so it is not one `UInt8` byte
    let same = b == 'ü';          // ERROR: character literal 'ü' is not ASCII, so it is not one `UInt8` byte
}

// Patterns are checked against the type they match, nested ones included.
func inPattern(b: UInt8, pair: (UInt8, Int64)) -> Int64 {
    let x = match b {
        'ü' => 1,                 // ERROR: character literal 'ü' is not ASCII, so it is not one `UInt8` byte
        'a'..='é' => 2,           // ERROR: character literal 'é' is not ASCII, so it is not one `UInt8` byte
        _ => 3,
    };
    match pair {
        ('ö', _) => 1,            // ERROR: character literal 'ö' is not ASCII, so it is not one `UInt8` byte
        _ => x,
    }
}

// The ASCII range and `Char` are fine.
func fine(b: UInt8) {
    let lo: UInt8 = '\0';
    let hi: UInt8 = '\x7F';
    let c: Char = 'é';
    let same = b == '#';
    let m = match c { 'é' => 1, _ => 2 };
}
