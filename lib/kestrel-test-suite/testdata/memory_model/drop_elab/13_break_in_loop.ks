// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: a `break` exits a loop with a live non-Copy local in
// scope. At Stage 7, all locals are function-scoped; the drop is
// emitted before the function's `Return`, after the loop has been
// exited. The fixture pins this; scope-tree drops would push it
// inside the loop body at the `break` edge.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func example() {
    let h = Handle(fd: 1);
    loop {
        break;
    }
}
