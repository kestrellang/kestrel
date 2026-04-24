// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.text.Char
import std.num.Int64

func classify(c: Char) -> Int64 {
    match c {
        'a'..<'z' => 1,  // a through y
        'z' => 2,        // exactly z
        _ => 0
    }
}

func main() -> lang.i64 {
    let lowerA: Char = 'a';
    let lowerY: Char = 'y';
    let lowerZ: Char = 'z';

    if classify(lowerA) != 1 { return 1 }
    if classify(lowerY) != 1 { return 2 }
    if classify(lowerZ) != 2 { return 3 }
    0
}
