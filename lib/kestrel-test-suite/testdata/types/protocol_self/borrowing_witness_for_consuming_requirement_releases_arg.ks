// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// `Iterable.iter()` is `consuming`, but a collection may satisfy it with a
// borrowing `iter()`. A `for` loop hands the requirement an owned value; the
// borrowing witness never releases it, so mono must. The captured `Marker`
// in the view's escaping predicate is dropped exactly once.

module Test

public var drops: Int64 = 0;

struct Marker: not Copyable {
    var ch: Char;
    func isSeparator(c: Char) -> Bool { c == self.ch }
    deinit { drops = drops + 1; }
}

func run() -> Int64 {
    let marker = Marker(ch: ' ');
    let text: String = "one two three";
    let view = text.split(where: { (c) in marker.isSeparator(c) });
    var n = 0;
    for s in view { n = n + 1; }
    n
}

@main
func main() -> lang.i64 {
    if run() != 3 { return 1 }
    if drops != 1 { return 2 }
    0
}
