// test: diagnostics
// stdlib: true

// Fuzz H12 follow-up: a `mutating` arg/receiver plus a `mutating` closure
// that writes the same variable in one call is E635. The same place passed
// to two `mutating` parameters is allowed (`swap(s, s)`). A writing closure
// counts as touching its captured locals whole (field-granular: fuzz M8).
module Test

import std.numeric.(Int64)

struct P {
    var a: Int64
    var b: Int64
    mutating func run(mutating f: mutating () -> ()) { f(); }
}

func t(mutating a: Int64, mutating b: Int64) { a = b; }
func w(mutating a: Int64, mutating f: mutating () -> ()) { f(); }

func main() {
    var x: Int64 = 1;
    var p = P(a: 1, b: 2);
    t(p.a, p.b);
    t(x, x);
    t(p.a, p.a);
    w(x, { x = x + 5 }); // ERROR(E635)
    w(p.a, { p.b = 3 }); // ERROR(E635)
    p.run({ p.a = 2 }); // ERROR(E635)
}
