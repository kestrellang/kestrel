// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    // ---- CharsView.isEmpty ----
    let s: std.text.String = "hello";
    if s.chars.isEmpty { return 1 }

    let empty = std.text.String();
    if empty.chars.isEmpty == false { return 2 }

    // ---- GraphemesView.isEmpty ----
    if s.graphemes.isEmpty { return 10 }
    if empty.graphemes.isEmpty == false { return 11 }

    // ---- GraphemesView.first ----
    if let .Some(g) = s.graphemes.first {
        if g.firstChar.isEqual(to: 'h') == false { return 12 }
    } else { return 13 }

    if let .Some(_) = empty.graphemes.first { return 14 }

    // ---- GraphemesView.last ----
    if let .Some(g) = s.graphemes.last {
        if g.firstChar.isEqual(to: 'o') == false { return 15 }
    } else { return 16 }

    if let .Some(_) = empty.graphemes.last { return 17 }

    // ---- GraphemesView.index(at:) ----
    if let .Some(idx) = s.graphemes.index(at: 0) {
        if idx.byteOffset != 0 { return 20 }
    } else { return 21 }

    if let .Some(idx) = s.graphemes.index(at: 3) {
        if idx.byteOffset != 3 { return 22 }
    } else { return 23 }

    // Out of bounds
    if let .Some(_) = s.graphemes.index(at: 10) { return 24 }

    // Multi-byte: é is 2 bytes
    let multi: std.text.String = "héllo";
    if let .Some(idx) = multi.graphemes.index(at: 2) {
        // 'h' = 1 byte, 'é' = 2 bytes, so grapheme 2 starts at byte 3
        if idx.byteOffset != 3 { return 25 }
    } else { return 26 }

    // ---- LinesView.isEmpty ----
    let lines: std.text.String = "a\nb\nc";
    if lines.lines.isEmpty { return 30 }
    if empty.lines.isEmpty == false { return 31 }

    // ---- LinesView.first ----
    if let .Some(first) = lines.lines.first {
        if first.isEqual(to: "a") == false { return 32 }
    } else { return 33 }

    if let .Some(_) = empty.lines.first { return 34 }

    // Single line (no terminator)
    let single: std.text.String = "hello";
    if let .Some(first) = single.lines.first {
        if first.isEqual(to: "hello") == false { return 35 }
    } else { return 36 }

    // ---- LinesView.index(at:) ----
    if let .Some(idx) = lines.lines.index(at: 0) {
        if idx.byteOffset != 0 { return 40 }
    } else { return 41 }

    if let .Some(idx) = lines.lines.index(at: 1) {
        // "a\n" is 2 bytes, so line 1 starts at byte 2
        if idx.byteOffset != 2 { return 42 }
    } else { return 43 }

    if let .Some(idx) = lines.lines.index(at: 2) {
        // "a\nb\n" is 4 bytes, so line 2 starts at byte 4
        if idx.byteOffset != 4 { return 44 }
    } else { return 45 }

    // Out of bounds
    if let .Some(_) = lines.lines.index(at: 10) { return 46 }

    // ---- LinesView.indexedIter() ----
    var lineCount: Int64 = 0;
    for entry in lines.lines.indexedIter() {
        let idx = entry.0;
        let line = entry.1;
        if lineCount == 0 {
            if idx.byteOffset != 0 { return 50 }
            if line.isEqual(to: "a") == false { return 51 }
        }
        if lineCount == 1 {
            if idx.byteOffset != 2 { return 52 }
            if line.isEqual(to: "b") == false { return 53 }
        }
        if lineCount == 2 {
            if idx.byteOffset != 4 { return 54 }
            if line.isEqual(to: "c") == false { return 55 }
        }
        lineCount = lineCount + 1
    }
    if lineCount != 3 { return 56 }

    0
}
