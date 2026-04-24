// test: diagnostics
// stdlib: true
// skip: Char range patterns with Matchable - LessOrEqual witness call needs investigation

module Test

import std.num.Int64

func classify(x: Int64) -> Int64 {
    match x {
        'a'..='z' => 1,
        _ => 0
    }
}

func main() -> lang.i64 {
    // 'a' = 97, 'z' = 122
    if classify(97) != 1 { return 1 }
    if classify(122) != 1 { return 2 }
    if classify(110) != 1 { return 3 }
    if classify(96) != 0 { return 4 }
    if classify(123) != 0 { return 5 }
    0
}
