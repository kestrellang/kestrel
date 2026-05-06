// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.text.String
import std.numeric.Int64

func classify(s: String) -> Int64 {
    match s {
        "fire" => 1,
        "water" => 2,
        "earth" => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    if classify("fire") != 1 { return 1 }
    if classify("water") != 2 { return 2 }
    if classify("earth") != 3 { return 3 }
    if classify("air") != 0 { return 4 }
    if classify("") != 0 { return 5 }
    0
}
