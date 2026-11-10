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

// Pattern position isn't checked yet (E121 has the same gap for integer
// patterns): `'ü'` here would truncate to 0xFC.
func inPattern(b: UInt8) -> Int64 {
    match b {
        'ü' => 1,                 // ERROR: character literal 'ü' is not ASCII, so it is not one `UInt8` byte
        _ => 2,
    }
}

// The ASCII range and `Char` are fine.
func fine(b: UInt8) {
    let lo: UInt8 = '\0';
    let hi: UInt8 = '\x7F';
    let c: Char = 'é';
    let same = b == '#';
}
