// test: diagnostics
// stdlib: true

module Test

import std.text.Char
import std.num.Int64

func isEmoji(c: Char) -> Int64 {
    match c {
        '\u{1F600}' => 1,
        '\u{1F601}' => 2,
        '\u{1F602}' => 3,
        _ => 0
    }
}

func main() -> lang.i64 {
    let grinning: Char = '\u{1F600}';
    let beaming: Char = '\u{1F601}';
    let joy: Char = '\u{1F602}';
    let letter: Char = 'a';

    if isEmoji(grinning) != 1 { return 1 }
    if isEmoji(beaming) != 2 { return 2 }
    if isEmoji(joy) != 3 { return 3 }
    if isEmoji(letter) != 0 { return 4 }
    0
}
