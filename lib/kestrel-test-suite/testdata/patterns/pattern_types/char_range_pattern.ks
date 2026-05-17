// test: diagnostics
// stdlib: true

module Test

import std.text.Char
import std.numeric.Int64

func classify(c: Char) -> Int64 {
    match c {
        'a'..='z' => 1,
        'A'..='Z' => 2,
        '0'..='9' => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    let lowerA: Char = 'a';
    let lowerZ: Char = 'z';
    let lowerM: Char = 'm';
    let upperA: Char = 'A';
    let upperZ: Char = 'Z';
    let digit0: Char = '0';
    let digit9: Char = '9';
    let space: Char = ' ';

    if classify(lowerA) != 1 { return 1 }
    if classify(lowerZ) != 1 { return 2 }
    if classify(lowerM) != 1 { return 3 }
    if classify(upperA) != 2 { return 4 }
    if classify(upperZ) != 2 { return 5 }
    if classify(digit0) != 3 { return 6 }
    if classify(digit9) != 3 { return 7 }
    if classify(space) != 0 { return 8 }
    0
}
