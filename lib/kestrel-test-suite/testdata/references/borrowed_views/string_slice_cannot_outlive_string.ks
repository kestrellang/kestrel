// test: diagnostics
// stdlib: true

// A StringSlice (and every text view) borrows its String, so it can't be
// returned past the string's lifetime or stored where it would outlive it.

module Test

import std.text.(String, StringSlice, CharsView)

func sliceOfLocal() -> StringSlice {
    let s: String = "hello";
    s.asSlice() // ERROR(E494)
}

func viewOfLocal() -> CharsView {
    let s: String = "hello";
    s.chars // ERROR(E494)
}

func sliceOfParam(s: String) -> StringSlice {
    s.asSlice()
}
