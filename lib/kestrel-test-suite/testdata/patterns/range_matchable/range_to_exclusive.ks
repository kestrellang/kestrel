// test: diagnostics
// stdlib: true
// ignore: Open-ended ranges need exhaustiveness checking updates

module Test

import std.num.Int64

func classify(x: Int64) -> Int64 {
    match x {
        ..<0 => 1,
        0 => 2,
        _ => 3
    }
}

func main() -> lang.i64 {
    if classify(-5) != 1 { return 1 }
    if classify(0) != 2 { return 2 }
    if classify(1) != 3 { return 3 }
    0
}
