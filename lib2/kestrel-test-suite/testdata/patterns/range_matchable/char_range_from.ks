// test: diagnostics
// stdlib: true
// ignore: Open-ended ranges need exhaustiveness checking updates

module Test

import std.text.Char
import std.num.Int64

func isUpperOrBeyond(c: Char) -> Int64 {
    match c {
        ..<'A' => 0,
        'A'.. => 1
    }
}

func main() -> lang.i64 {
    let at: Char = '@';
    let upperA: Char = 'A';
    let upperZ: Char = 'Z';
    let lowerA: Char = 'a';

    if isUpperOrBeyond(at) != 0 { return 1 }
    if isUpperOrBeyond(upperA) != 1 { return 2 }
    if isUpperOrBeyond(upperZ) != 1 { return 3 }
    if isUpperOrBeyond(lowerA) != 1 { return 4 }
    0
}
