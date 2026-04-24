// test: diagnostics
// stdlib: true

module Test

import std.text.Char
import std.num.Int64

func classify(c: Char) -> Int64 {
    match c {
        'a'..<'z' => 1,
        'z' => 2,
        _ => 0
    }
}

func main() -> lang.i64 {
    let lowerA: Char = 'a';
    let lowerY: Char = 'y';
    let lowerZ: Char = 'z';
    let other: Char = '!';

    if classify(lowerA) != 1 { return 1 }
    if classify(lowerY) != 1 { return 2 }
    if classify(lowerZ) != 2 { return 3 }
    if classify(other) != 0 { return 4 }
    0
}
