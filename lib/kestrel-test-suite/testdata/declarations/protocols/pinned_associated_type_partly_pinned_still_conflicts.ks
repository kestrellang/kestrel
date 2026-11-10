// test: diagnostics
// stdlib: false
module Test

// Pinning only one of two same-named associated types leaves the other
// ambiguous.
protocol A {
    type Element
}

protocol B {
    type Element
}

protocol C: A, B where Self.A.Element = Self { // ERROR: conflicting associated type 'Element'
}
