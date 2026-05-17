// test: diagnostics
// stdlib: true

module Test

import std.text.String
import std.numeric.Int64

// E316 fires once a `match` on `String` exceeds 4 literal arms — past that
// point each additional arm is another byte-equality call at runtime, so the
// compiler nudges the user toward an `if`/`else if` chain.
func tag(s: String) -> Int64 {
    match s { // WARN: match on `String` with 5 literal arms does byte-equality per arm
        "a" => 1,
        "b" => 2,
        "c" => 3,
        "d" => 4,
        "e" => 5,
        _ => 0
    }
}
