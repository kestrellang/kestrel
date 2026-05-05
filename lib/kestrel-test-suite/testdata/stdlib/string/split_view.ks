// test: execution
// stdlib: true

module Test

func main() -> lang.i64 {
    // Verify extend Str methods work on StringSlice
    let s: std.text.String = "hello";
    let slice = s.asSlice();
    if slice.byteCount != 5 { return 99 }

    // ---- SplitView via Str protocol (separator) ----
    let csv: std.text.String = "a,b,c";
    let view = csv.asSlice().split(",");

    // count
    if view.count != 3 { return 1 }

    // first / last
    if let .Some(f) = view.first {
        if f.toOwned().isEqual(to: "a") == false { return 2 }
    } else { return 3 }

    if let .Some(l) = view.last {
        if l.toOwned().isEqual(to: "c") == false { return 4 }
    } else { return 5 }

    // collect
    let parts = view.collect();
    if parts.count != 3 { return 6 }
    if parts(unchecked: 0).toOwned().isEqual(to: "a") == false { return 7 }
    if parts(unchecked: 1).toOwned().isEqual(to: "b") == false { return 8 }
    if parts(unchecked: 2).toOwned().isEqual(to: "c") == false { return 9 }

    // No separator found — single segment
    let noSep: std.text.String = "hello";
    let noSepView = noSep.asSlice().split(",");
    if noSepView.count != 1 { return 10 }
    if let .Some(only) = noSepView.first {
        if only.toOwned().isEqual(to: "hello") == false { return 11 }
    } else { return 12 }

    // Adjacent separators — empty segments preserved
    let adj: std.text.String = "a,,b";
    let adjParts = adj.asSlice().split(",").collect();
    if adjParts.count != 3 { return 13 }
    if adjParts(unchecked: 1).toOwned().isEqual(to: "") == false { return 14 }

    // Multi-byte separator
    let multi: std.text.String = "one::two::three";
    let multiParts = multi.asSlice().split("::").collect();
    if multiParts.count != 3 { return 15 }
    if multiParts(unchecked: 0).toOwned().isEqual(to: "one") == false { return 16 }
    if multiParts(unchecked: 2).toOwned().isEqual(to: "three") == false { return 17 }

    // Empty separator — splits per code point
    let chars: std.text.String = "abc";
    let charParts = chars.asSlice().split("").collect();
    if charParts.count != 3 { return 18 }
    if charParts(unchecked: 0).toOwned().isEqual(to: "a") == false { return 19 }
    if charParts(unchecked: 2).toOwned().isEqual(to: "c") == false { return 20 }

    // isEmpty
    let empty: std.text.String = "";
    if empty.asSlice().split(",").isEmpty == false { return 21 }

    // for-in iteration
    var count: Int64 = 0;
    for segment in csv.asSlice().split(",") {
        count = count + 1
    }
    if count != 3 { return 22 }

    // ---- SplitWhereView (predicate) ----
    let ws: std.text.String = "hello world";
    let wsView = ws.asSlice().split(matching: { (c) in c.isWhitespace() });
    if wsView.count != 2 { return 30 }

    let wsParts = wsView.collect();
    if wsParts(unchecked: 0).toOwned().isEqual(to: "hello") == false { return 31 }
    if wsParts(unchecked: 1).toOwned().isEqual(to: "world") == false { return 32 }

    // Predicate split — first/last
    if let .Some(first) = wsView.first {
        if first.toOwned().isEqual(to: "hello") == false { return 33 }
    } else { return 34 }

    if let .Some(last) = wsView.last {
        if last.toOwned().isEqual(to: "world") == false { return 35 }
    } else { return 36 }

    // Predicate split on digits
    let mixed: std.text.String = "abc1def2ghi";
    let digitParts = mixed.asSlice().split(matching: { (c) in c.isAsciiDigit() }).collect();
    if digitParts.count != 3 { return 37 }
    if digitParts(unchecked: 0).toOwned().isEqual(to: "abc") == false { return 38 }
    if digitParts(unchecked: 1).toOwned().isEqual(to: "def") == false { return 39 }
    if digitParts(unchecked: 2).toOwned().isEqual(to: "ghi") == false { return 40 }

    0
}
