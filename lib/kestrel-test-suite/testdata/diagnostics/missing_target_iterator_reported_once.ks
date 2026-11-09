// test: diagnostics
// stdlib: true

// A type that declares `Iterable` without `type TargetIterator` gets E455
// only. A `for` loop over it used to add an E102 pointing into the standard
// library's `iterator.ks` and an "expected _ got _" E109 at the loop.
module Test

struct Bag: Iterable { // ERROR(E455)
    type Item = Int64
    var xs: [Int64]
    func iter() -> ArrayIterator[Int64] { self.xs.iter() }
}

func b() {
    for x in Bag(xs: [1]) {}
}
