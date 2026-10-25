// test: execution
// stdlib: true
// expect-exit: 0

// Regression (#132): `Array(from:)` with an Iterator argument, whose Iterable
// conformance comes from the blanket `extend Iterator: Iterable`, ICEd in
// post-mono verify. The extension's associated-type bindings
// (`type Iterable.TargetIterator = Self`) lower `Self` to a TypeParam keyed on
// the extension's TARGET protocol (Iterator), but `replace_self_type`
// substituted only the witnessed protocol's key (Iterable) — a silent no-op
// that leaked the raw TypeParam into monomorphization.
//
// See lib/kestrel-mir-lower/src/items/witness_lower.rs::replace_self_type.

module Test

@main
func main() -> lang.i64 {
    // Iterator via String bytes (the original #132 repro).
    let a = Array(from: "salt".bytes.iter());
    if a.count != 4 { return 1 }
    if a(0) != 115 { return 2 }  // 's'

    // Iterator obtained from an Array's own iterator.
    let nums = [10, 20, 30];
    let b = Array(from: nums.iter());
    if b.count != 3 { return 3 }
    if b(2) != 30 { return 4 }

    0
}
