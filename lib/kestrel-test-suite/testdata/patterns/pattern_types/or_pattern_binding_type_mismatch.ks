// test: diagnostics
// stdlib: true
//
// Every or-pattern alternative must bind each name at the same type. The
// alternatives share one local per name, and before this check the last
// alternative's type silently won: this program compiled and segfaulted,
// reading a String payload as an Int64.

module Test

import std.numeric.(Int64)
import std.text.(String)
import std.io.stdio.(print)

enum E {
    case A(Int64)
    case B(String)
}

func f(e: E) -> Int64 {
    match e {
        .A(v) or .B(v) => { print("\(v)"); 0 }, // ERROR: variable 'v' has different types in or-pattern alternatives
    }
}

// Same type in every alternative is fine.
enum F {
    case C(Int64)
    case D(Int64)
}

func g(x: F) -> Int64 {
    match x {
        .C(v) or .D(v) => v,
    }
}
