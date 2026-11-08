// test: execution
// expect-exit: 0

module Test

struct V: Equatable {
    var x: Int64
    func isEqual(to other: V) -> Bool { self.x == other.x }
}

// An implementation of a sugar-only requirement may forward to another one.
struct Label: ExpressibleByStringLiteral {
    var text: String
    init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.text = String(stringLiteral: ptr, length);
    }
}

func operators() -> Int64 {
    var a = 7;
    a %= 3;
    a += 1;
    let b = a == 2 and a != 3;
    let o: Int64? = .None;
    let c = o ?? 4;
    var total = 0;
    for i in 0..<3 { total += i }
    if V(x: 1) == V(x: 1) and V(x: 1) != V(x: 2) and b { total += 1 }
    match [1, 2, 3] {
        [first, ..rest] => { total += first },
        _ => {},
    }
    total + c + (a & 1) + (a << 1)
}

func tryIt() -> Int64? {
    let o: Int64? = .Some(1);
    let v = try o;
    .Some(v + 1)
}

@main
func main() -> Int64 {
    // 3 + 1 + 1 + 4 + 0 + 4 = 13
    if operators() != 13 { return 1 }
    if tryIt() != .Some(2) { return 2 }
    let xs = [10, 20, 30];
    if xs(1) != 20 { return 3 }
    let n = 5;
    if "n=\(n)" != "n=5" { return 4 }
    let l: Label = "hi";
    if l.text != "hi" { return 5 }
    0
}
