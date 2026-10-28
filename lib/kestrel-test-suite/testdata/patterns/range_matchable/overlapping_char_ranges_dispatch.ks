// test: execution
// stdlib: true
// expect-exit: 0
//
// F1 — the char version of overlapping_int_ranges_dispatch.ks. 'q' is outside
// 'a'..='m' and used to be routed to arm 1 anyway.

module Test

import std.text.Char

func classify(c: Char) -> Int64 {
    match c {
        'a'..='m' => 1,
        'f'..='z' => 2,
        _ => 0
    }
}

@main
func main() -> lang.i32 {
    let c: Char = 'c';
    let h: Char = 'h';
    let q: Char = 'q';
    let bang: Char = '!';
    if classify(c) != 1 { return 1 }
    if classify(h) != 1 { return 2 }
    if classify(q) != 2 { return 3 }
    if classify(bang) != 0 { return 4 }
    0
}
