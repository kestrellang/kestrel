// test: diagnostics
// stdlib: true

// An array literal's element is a generic `T` too: a slice can't be stored
// by value, and a `&StringSlice` element would not be Static. The element
// decays to the unsized pointee so the error names it as unsized.

module Test

import std.text.(String)

func use(s: String) -> Int64 {
    let xs = [s.trimmed()]; // ERROR(E107)
    0
}
