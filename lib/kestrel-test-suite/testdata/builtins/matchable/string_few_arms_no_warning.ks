// test: diagnostics
// stdlib: true

module Test

import std.text.String
import std.numeric.Int64

// A short match on string literals draws no warning.
func tag(s: String) -> Int64 {
    match s {
        "a" => 1,
        "b" => 2,
        "c" => 3,
        "d" => 4,
        _ => 0
    }
}
