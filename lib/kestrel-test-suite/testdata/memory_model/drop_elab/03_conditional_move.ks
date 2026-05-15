// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: a non-Copy local is consumed on one branch but not the
// other. DropElab must guard the drop with an `_init_h` flag so the
// branch that didn't move it runs its destructor and the branch that
// did move it doesn't double-free.

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
    let h = Handle(fd: 1);
    if cond {
        consume(h);
    }
}
