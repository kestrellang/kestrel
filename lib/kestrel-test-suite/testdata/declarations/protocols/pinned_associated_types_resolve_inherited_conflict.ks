// test: diagnostics
// stdlib: false
module Test

// Two parents declare `Element`; the child's `where` clause says what each
// one is, so the name no longer conflicts (contrast
// diamond_inheritance_associated_type_conflict.ks).
protocol A {
    type Element
}

protocol B {
    type Element
}

protocol C: A, B where Self.A.Element = Self, Self.B.Element = Self {
}

// A refinement inherits the pins.
protocol D: C {
}
