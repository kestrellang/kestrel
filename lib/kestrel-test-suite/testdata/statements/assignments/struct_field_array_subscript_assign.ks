// test: execution
// stdlib: true
// expect-exit: 0

// Regression: `obj.field(i) = v` where `field` is a stored Array[T] field on
// a struct used to lower as a subscript-getter call + dead local-write — the
// setter was never invoked and the mutation silently vanished. Both writes
// through a `mutating self` method and direct `h.data(i) = v` from outside
// hit the same broken path because both surface as `HirExpr::MethodCall`,
// which `try_lower_setter_assign` didn't recognise.
//
// See lib/kestrel-mir-lower/src/body_lower.rs::try_lower_setter_assign.

module Test

struct Holder: Cloneable {
    var data: Array[Int64]

    init() {
        self.data = Array[Int64](repeating: 0, count: 4)
    }

    func clone() -> Holder {
        var h = Holder();
        h.data = self.data.clone();
        h
    }

    mutating func light(at i: Int64) {
        self.data(i) = 1
    }
}

func sum(h: Holder) -> Int64 {
    var s: Int64 = 0;
    var i: Int64 = 0;
    while i < h.data.count {
        s = s + h.data(i);
        i = i + 1;
    }
    s
}

@main
func main() -> lang.i64 {
    var h = Holder();
    if sum(h) != 0 { return 1 }

    // Subscript-write inside a `mutating` method — the path that surfaced
    // this bug in Game of Life.
    h.light(at: 0);
    h.light(at: 2);
    if sum(h) != 2 { return 2 }

    // Direct subscript-write on a struct field from outside the type.
    h.data(3) = 5;
    if sum(h) != 7 { return 3 }

    0
}
