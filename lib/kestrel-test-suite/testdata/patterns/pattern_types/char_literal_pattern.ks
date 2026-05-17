// test: diagnostics
// stdlib: true

module Test

import std.text.Char
import std.numeric.Int64

func classify(c: Char) -> Int64 {
    match c {
        'a' => 1,
        'b' => 2,
        'c' => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    let a: Char = 'a';
    let b: Char = 'b';
    let c: Char = 'c';
    let d: Char = 'd';

    if classify(a) != 1 { return 1 }
    if classify(b) != 2 { return 2 }
    if classify(c) != 3 { return 3 }
    if classify(d) != 0 { return 4 }
    0
}
