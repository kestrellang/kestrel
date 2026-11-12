// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: true false true\ntrue 7 b,cde None Some(e) Some(e)\nde a,b\n2\ntrue 0\nd a\n
//
// `s.isAscii` checks every byte; `s.ascii` gives an `AsciiView` only for
// ASCII text, indexing by character in O(1) with every access mode the
// other views have. A sub-view indexes from its own start.

module Test

@main
func main() {
    let mixed: String = "caf\u{00E9}";
    print("\("Hello".isAscii) \(mixed.isAscii) \("".isAscii)\n");

    let s: String = "a,b,cde";
    guard let .Some(t) = s.ascii else { return }
    print("\(t(1) == ',') \(t.count) \(t(2..).toOwned()) \(t(checked: 9)) \(t(wrapped: -1)) \(t(clamped: 100))\n");
    print("\(t(within: 5..<99).toOwned()) \(t.substring(..=2))\n");

    var commas = 0;
    for ch in t {
        if ch == ',' { commas = commas + 1 }
    }
    print("\(commas)\n");

    print("\(mixed.ascii.isNone) \("".ascii!.count)\n");

    let sub = t(4..<7);
    let first = unsafe t(unchecked: 0);
    print("\(sub(1)) \(first)\n");
}
