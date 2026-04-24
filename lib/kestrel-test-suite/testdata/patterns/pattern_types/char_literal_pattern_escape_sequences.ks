// test: diagnostics
// stdlib: true

module Test

import std.text.Char
import std.num.Int64

func classify(c: Char) -> Int64 {
    match c {
        '\n' => 1,
        '\t' => 2,
        '\\' => 3,
        '\'' => 4,
        '\0' => 5,
        _ => 0
    }
}

func main() -> lang.i64 {
    let newline: Char = '\n';
    let tab: Char = '\t';
    let backslash: Char = '\\';
    let quote: Char = '\'';
    let nul: Char = '\0';
    let other: Char = 'x';

    if classify(newline) != 1 { return 1 }
    if classify(tab) != 2 { return 2 }
    if classify(backslash) != 3 { return 3 }
    if classify(quote) != 4 { return 4 }
    if classify(nul) != 5 { return 5 }
    if classify(other) != 0 { return 6 }
    0
}
