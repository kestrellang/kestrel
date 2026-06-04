// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: a non-Copy local is assigned in both arms of an `if`.
// The dataflow joins to `DefinitelyInit` at the join point, so the
// drop is unconditional (not flag-guarded) at the function's return.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func example(cond: lang.i1) {
    var h = Handle(fd: 0);
    if cond {
        h = Handle(fd: 1);
    } else {
        h = Handle(fd: 2);
    }
}
