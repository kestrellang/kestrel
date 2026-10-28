// test: diagnostics

// A witness's receiver must match the requirement's exactly. A `mutating`
// method cannot satisfy a non-mutating requirement (generic callers pass
// `self` as a read-only borrow, so it would write through a `let`), and a
// non-mutating method no longer satisfies a `mutating` one either.

module Test

protocol P { func bump() }
protocol Q { mutating func bump() }

struct S: P {
    var n: Int64
    mutating func bump() { self.n = self.n + 1; } // ERROR(E477)
}

struct T: Q {
    var n: Int64
    func bump() { } // ERROR(E477)
}
