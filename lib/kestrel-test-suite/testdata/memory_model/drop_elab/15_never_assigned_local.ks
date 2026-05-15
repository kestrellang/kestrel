// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: a non-Copy local is declared (via parameter pattern with a
// `let`-style binding that isn't initialised on every path) and the
// dataflow proves it's `Uninit` at the return. DropElab's "is the path
// MaybeInit at exit?" check trims the drop so no destructor runs on a
// never-constructed value.
//
// Built by branching on `cond`: the local is only assigned in one arm
// of an `if/else` that exits before reaching the function return, so
// at the return point the path is still uninit.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func consume(consuming h: Handle) {}

func example(cond: lang.i1) {
    if cond {
        let h = Handle(fd: 1);
        consume(h);
        return;
    }
    // Reached only when `cond` is false. No `Handle` was ever
    // constructed in this body's scope at this point.
    return;
}
