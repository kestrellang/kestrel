// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: a struct of only trivially-copyable fields, no `not Copyable`
// annotation. Its `CopyBehavior` is `Bitwise`, so DropElab skips it
// entirely — no drops in the MIR text.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func example() {
    let p = Point(x: 1, y: 2);
}
