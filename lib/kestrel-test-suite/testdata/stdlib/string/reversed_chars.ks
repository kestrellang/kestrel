// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    // ---- Basic ASCII ----
    let s: std.text.String = "abc";
    let rev = s.chars.reversed;

    // first() returns last char of source
    if let .Some(c) = rev.first {
        if c.isEqual(to: 'c') == false { return 1 }
    } else { return 2 }

    // count
    if rev.count != 3 { return 3 }

    // isEmpty
    if rev.isEmpty { return 4 }

    // Collect reversed chars
    var chars = std.collections.Array[std.text.Char]();
    for c in rev {
        chars.append(c)
    }
    if chars.count != 3 { return 5 }
    if chars(unchecked: 0).isEqual(to: 'c') == false { return 6 }
    if chars(unchecked: 1).isEqual(to: 'b') == false { return 7 }
    if chars(unchecked: 2).isEqual(to: 'a') == false { return 8 }

    // ---- Empty string ----
    let empty: std.text.String = "";
    let emptyRev = empty.chars.reversed;
    if emptyRev.isEmpty == false { return 9 }
    if emptyRev.count != 0 { return 10 }
    if let .Some(_) = emptyRev.first { return 11 }

    // ---- Single char ----
    let single: std.text.String = "x";
    var singleChars = std.collections.Array[std.text.Char]();
    for c in single.chars.reversed {
        singleChars.append(c)
    }
    if singleChars.count != 1 { return 12 }
    if singleChars(unchecked: 0).isEqual(to: 'x') == false { return 13 }

    // ---- Multi-byte UTF-8 ----
    let uni: std.text.String = "a\u{00E9}b";
    var uniChars = std.collections.Array[std.text.Char]();
    for c in uni.chars.reversed {
        uniChars.append(c)
    }
    if uniChars.count != 3 { return 14 }
    if uniChars(unchecked: 0).isEqual(to: 'b') == false { return 15 }
    if uniChars(unchecked: 1).isEqual(to: '\u{00E9}') == false { return 16 }
    if uniChars(unchecked: 2).isEqual(to: 'a') == false { return 17 }

    // ---- 4-byte chars ----
    let emoji: std.text.String = "A\u{1F600}Z";
    var emojiChars = std.collections.Array[std.text.Char]();
    for c in emoji.chars.reversed {
        emojiChars.append(c)
    }
    if emojiChars.count != 3 { return 18 }
    if emojiChars(unchecked: 0).isEqual(to: 'Z') == false { return 19 }
    if emojiChars(unchecked: 2).isEqual(to: 'A') == false { return 20 }

    // ---- Works on StringSlice too ----
    let slice = "hello".asSlice();
    var sliceChars = std.collections.Array[std.text.Char]();
    for c in slice.chars.reversed {
        sliceChars.append(c)
    }
    if sliceChars.count != 5 { return 21 }
    if sliceChars(unchecked: 0).isEqual(to: 'o') == false { return 22 }
    if sliceChars(unchecked: 4).isEqual(to: 'h') == false { return 23 }

    0
}
