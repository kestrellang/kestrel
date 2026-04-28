// test: diagnostics
// stdlib: true
// ignore: Open-ended ranges need exhaustiveness checking updates

module Test

import std.numeric.Int64

func classify(x: Int64) -> Int64 {
    match x {
        ..<0 => 1,
        0..=59 => 2,
        60.. => 3
    }
}

func main() -> lang.i64 {
    if classify(-1) != 1 { return 1 }
    if classify(0) != 2 { return 2 }
    if classify(59) != 2 { return 3 }
    if classify(60) != 3 { return 4 }
    if classify(100) != 3 { return 5 }
    0
}
