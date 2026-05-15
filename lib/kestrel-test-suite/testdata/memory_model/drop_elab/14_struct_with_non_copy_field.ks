// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: a struct holding another `not Copyable` struct as a field.
// Both the outer and (eventually, when field-level partials land) the
// inner field need destruction. At Stage 7 move-paths are root-only,
// so the drop is emitted for the outer local only. The fixture pins
// that behavior; field-level extension is a follow-up.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Inner: not Copyable {
    var x: lang.i64
    deinit {}
}

struct Outer: not Copyable {
    var inner: Inner
    deinit {}
}

func example() {
    let o = Outer(inner: Inner(x: 1));
}
