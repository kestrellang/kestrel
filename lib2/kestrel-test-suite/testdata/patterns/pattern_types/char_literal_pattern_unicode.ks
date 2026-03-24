// test: diagnostics
// stdlib: true

module Test

import std.text.Char
import std.num.Int64

func classify(c: Char) -> Int64 {
    match c {
        'Ω' => 1,
        '日' => 2,
        '本' => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    let omega: Char = 'Ω';
    let sun: Char = '日';
    let book: Char = '本';
    let ascii: Char = 'a';

    if classify(omega) != 1 { return 1 }
    if classify(sun) != 2 { return 2 }
    if classify(book) != 3 { return 3 }
    if classify(ascii) != 0 { return 4 }
    0
}
