// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.text.String
import std.numeric.Int64

// Verifies that empty-string and multi-byte (non-ASCII) literals are
// matched by byte equality, not by UTF-8 code-point indexing or length-only
// comparison.
func tag(s: String) -> Int64 {
    match s {
        "" => 1,
        "café" => 2,
        "naïve" => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    if tag("") != 1 { return 1 }
    if tag("café") != 2 { return 2 }
    if tag("naïve") != 3 { return 3 }
    // Same byte-count as "café" but different bytes — must not collide.
    if tag("cafe") != 0 { return 4 }
    // Same prefix, different length — must not collide.
    if tag("c") != 0 { return 5 }
    0
}
