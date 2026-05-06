// test: diagnostics
// stdlib: true

module Test

import std.text.String
import std.numeric.Int64

// Four literal arms is below the E316 threshold — no warning should fire,
// since small finite-tag matches are the intended use case.
func tag(s: String) -> Int64 {
    match s {
        "a" => 1,
        "b" => 2,
        "c" => 3,
        "d" => 4,
        _ => 0
    }
}
