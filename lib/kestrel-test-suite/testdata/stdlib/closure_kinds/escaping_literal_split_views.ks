// test: execution
// stdlib: true
// expect-exit: 0

// The two non-iterator lazy builders that store a callback —
// `ArraySlice.split(where:)` and `Str.split(where:)` — take `escaping` closures
// (closures-stdlib-audit.md). Their view types own the snapshot, so mutating
// the captured source var after construction cannot change what the view
// yields (docs/design/closures.md, "Capture Rules").
module Test

@main
func main() -> lang.i64 {
    // Str.split(where:) — snapshot of `sepChar`
    var sepChar: Char = ' ';
    let text: String = "one two three";
    let words = text.split(where: { (c) in c == sepChar });
    sepChar = 'e';
    if words.count != 3 { return 1 }
    let wordParts = words.collect();
    if wordParts.count != 3 { return 2 }
    if wordParts(unchecked: 0).toOwned().isEqual(to: "one") == false { return 3 }
    if wordParts(unchecked: 1).toOwned().isEqual(to: "two") == false { return 4 }
    if wordParts(unchecked: 2).toOwned().isEqual(to: "three") == false { return 5 }

    // ArraySlice.split(where:) — snapshot of `marker`
    var marker: Int64 = -1;
    let arr = [1, -1, 2, 3, -1, 4];
    let segments = arr.split(where: { (x) in x == marker });
    marker = 0;
    if segments.count != 3 { return 6 }
    let segs = segments.toArray();
    if segs.count != 3 { return 7 }
    if segs(unchecked: 0).count != 1 { return 8 }
    if segs(unchecked: 1).count != 2 { return 9 }
    if segs(unchecked: 2).count != 1 { return 10 }

    0
}
