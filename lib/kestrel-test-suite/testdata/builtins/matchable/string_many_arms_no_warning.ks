// test: diagnostics
// stdlib: true

module Test

import std.text.String
import std.numeric.Int64

// A long match on string literals is ordinary dispatch code and draws no
// warning: an `if`/`else if` chain on `==` costs the same per arm.
func tag(s: String) -> Int64 {
    match s {
        "a" => 1,
        "b" => 2,
        "c" => 3,
        "d" => 4,
        "e" => 5,
        "f" => 6,
        _ => 0
    }
}
