// test: execution
// stdlib: true

// BytesView.toString() must always produce a well-formed String:
// valid UTF-8 round-trips exactly, and a sub-view cut mid-codepoint
// replaces the broken sequence with U+FFFD instead of minting an
// invalid String.

module Test

@main
func main() -> lang.i64 {
    // ---- valid UTF-8 round-trips ----
    let s: std.text.String = "héllo";
    let bytes = s.bytes;
    if bytes.count != 6 { return 1 }
    let round = bytes.toString();
    if round.isEqual(to: "héllo") == false { return 2 }

    // ---- whole-view of ASCII ----
    let ascii: std.text.String = "abc";
    if ascii.bytes.toString().isEqual(to: "abc") == false { return 3 }

    // ---- mid-codepoint slice is lossy, not invalid ----
    // "héllo" bytes: h=0x68, é=0xC3 0xA9, l, l, o
    // Slicing 0..<2 cuts é in half: [0x68, 0xC3].
    let cut = s.bytes(0..<2);
    if cut.count != 2 { return 4 }
    let repaired = cut.toString();
    // "h" + U+FFFD — 1 byte + 3 bytes = 4 bytes.
    if repaired.bytes.count != 4 { return 5 }
    if repaired.isEqual(to: "h\u{FFFD}") == false { return 6 }

    // ---- slice starting mid-codepoint ----
    let tail = s.bytes(1..<6);
    let repairedTail = tail.toString();
    // 0xC3 alone? No — 1..<6 is [0xC3,0xA9,l,l,o] which decodes fine as "éllo".
    if repairedTail.isEqual(to: "éllo") == false { return 7 }

    // both halves of é broken apart
    let lone = s.bytes(2..<3);   // [0xA9] — bare continuation byte
    if lone.toString().isEqual(to: "\u{FFFD}") == false { return 8 }

    0
}
