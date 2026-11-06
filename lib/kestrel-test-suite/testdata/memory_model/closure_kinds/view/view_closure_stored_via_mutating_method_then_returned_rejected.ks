// test: diagnostics
// stdlib: true

// A view closure capturing a local, stored into a value by one of the
// value's `mutating` methods, after which the value is returned (how Perch
// stores middleware). View closures are not `Static`
// (docs/design/closure-fields.md), so the array of them is rejected at its
// declaration (E107) and the escape can no longer be written; the fix is an
// `escaping` element type, whose literal snapshots `k`.

module Test

struct Reg {
    var hs: [(Int64) -> Int64]; // ERROR(E107)
    mutating func add(h: (Int64) -> Int64) { self.hs.append(h); }
}

func make() -> Reg {
    var r = Reg(hs: []);
    let k: Int64 = 5;
    r.add({ it + k });
    r
}
