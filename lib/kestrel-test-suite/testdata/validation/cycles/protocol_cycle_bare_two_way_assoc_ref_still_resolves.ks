// test: diagnostics
// stdlib: false

module Test

// Regression guard for the cycle-guard fix: the associated type referenced
// through the cycle *does* exist, so resolution must still succeed. The
// cycle is reported (E459), but `T.Item` resolves to `Item` on `B` and no
// "cannot find type" is emitted.
//
// This pins the re-anchored resolution context. The inherited walk resolves
// each conformance path relative to the *declaring* protocol's parent scope;
// if that anchor drifts (as the old duplicated walk's did, climbing one extra
// ancestor per level), `B` stops being visible and this regresses to
// "cannot find type 'T.Item'".
protocol A: B {} // ERROR(E459)
protocol B: A {
    type Item
}

func take[T](x: T.Item) -> () where T: A {}
