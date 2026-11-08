// test: execution
// stdlib: true
// expect-stdout: 42 7 7\n

module Test

// An alias's initializer can be called through its module path, like the
// struct it names. `Test.C(7)` used to resolve as a subscript on `Counter`
// (E102 no matching subscript), as did `std.collections.FastHasher()`.

struct Counter {
    var count: Int64

    init(c: Int64) {
        self.count = c;
    }
}

type C = Counter

@main
func main() {
    let a = C(42);
    let b = Test.C(7);
    var h = std.collections.FastHasher();
    var g = FastHasher();
    let same = h.finish() == g.finish();
    print("\(a.count) \(b.count) \(if same { 7 } else { 0 })\n");
}
