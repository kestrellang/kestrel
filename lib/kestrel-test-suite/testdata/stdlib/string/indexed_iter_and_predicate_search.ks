// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    // ---- CharsView.firstIndex(where:) ----
    let s: std.text.String = "hello world";
    let spaceIdx = s.chars.firstIndex(where: { (c) in c.isWhitespace });
    if let .Some(idx) = spaceIdx {
        if idx.byteOffset != 5 { return 1 }
    } else { return 2 }

    // No match
    let noMatch = s.chars.firstIndex(where: { (c) in c.isAsciiDigit });
    if let .Some(_) = noMatch { return 3 }

    // ---- CharsView.lastIndex(where:) ----
    let s2: std.text.String = "a b c d";
    let lastSpace = s2.chars.lastIndex(where: { (c) in c.isWhitespace });
    if let .Some(idx) = lastSpace {
        if idx.byteOffset != 5 { return 4 }
    } else { return 5 }

    // ---- Multi-byte: predicate search ----
    let uni: std.text.String = "hi\u{00E9}lo";
    let accent = uni.chars.firstIndex(where: { (c) in c.value() > 127 });
    if let .Some(idx) = accent {
        if idx.byteOffset != 2 { return 6 }
    } else { return 7 }

    // ---- GraphemesView.firstIndex(where:) ----
    let g: std.text.String = "abc def";
    let gIdx = g.graphemes.firstIndex(where: { (gr) in
        gr.firstChar.isWhitespace
    });
    if let .Some(idx) = gIdx {
        if idx.byteOffset != 3 { return 8 }
    } else { return 9 }

    // ---- CharsView.indexedIter() ----
    let ascii: std.text.String = "abc";
    var charCount: Int64 = 0;
    for (idx, c) in ascii.chars.indexedIter() {
        if charCount == 0 {
            if idx.byteOffset != 0 { return 10 }
        }
        if charCount == 2 {
            if idx.byteOffset != 2 { return 11 }
        }
        charCount = charCount + 1
    }
    if charCount != 3 { return 12 }

    // Multi-byte indexedIter — byte offsets reflect actual positions
    let mb: std.text.String = "a\u{00E9}b";
    var offsets = std.collections.Array[Int64]();
    for (idx, c) in mb.chars.indexedIter() {
        offsets.append(idx.byteOffset)
    }
    if offsets.count != 3 { return 13 }
    if offsets(unchecked: 0) != 0 { return 14 }
    if offsets(unchecked: 1) != 1 { return 15 }
    if offsets(unchecked: 2) != 3 { return 16 }

    // ---- GraphemesView.indexedIter() ----
    let gs: std.text.String = "ab cd";
    var gOffsets = std.collections.Array[Int64]();
    for (idx, g) in gs.graphemes.indexedIter() {
        gOffsets.append(idx.byteOffset)
    }
    if gOffsets.count != 5 { return 17 }
    if gOffsets(unchecked: 0) != 0 { return 18 }
    if gOffsets(unchecked: 2) != 2 { return 19 }
    if gOffsets(unchecked: 3) != 3 { return 20 }

    0
}
