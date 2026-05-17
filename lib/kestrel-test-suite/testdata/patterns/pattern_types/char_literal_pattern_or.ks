// test: diagnostics
// stdlib: true

module Test

import std.text.Char
import std.numeric.Int64

func classifyCaseInsensitive(c: Char) -> Int64 {
    match c {
        'a' or 'A' => 1,
        'b' or 'B' => 2,
        'c' or 'C' => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    let lowerA: Char = 'a';
    let upperA: Char = 'A';
    let lowerB: Char = 'b';
    let upperB: Char = 'B';
    let other: Char = 'x';

    if classifyCaseInsensitive(lowerA) != 1 { return 1 }
    if classifyCaseInsensitive(upperA) != 1 { return 2 }
    if classifyCaseInsensitive(lowerB) != 2 { return 3 }
    if classifyCaseInsensitive(upperB) != 2 { return 4 }
    if classifyCaseInsensitive(other) != 0 { return 5 }
    0
}
