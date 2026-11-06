// test: diagnostics
// stdlib: true

// Conformance checking resolves an associated type's binding to see whether
// it meets the protocol's bound. `[Int64]` there must mean the stdlib's Array
// (through its lang item), exactly as HIR lowering reads it. A user type that
// happens to be named `Array` must not capture it.
//
// The analyzer used to look sugar up by its plain name ("Array") in the user's
// scope, so this file's `Array` won, and the binding was reported as not
// satisfying `Marker` even though `[Int64]` does.

module Main

protocol Marker { }
extend [T]: Marker { }

struct Array { }

protocol HasItems {
    type Items;
}
protocol MarkedItems: HasItems where HasItems.Items: Marker { }

struct Holder: MarkedItems {
    type Items = [Int64];
}
