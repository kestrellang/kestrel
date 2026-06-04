// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: a nested block expression with locals at multiple "syntactic
// scopes". At Stage 7, MIR doesn't carry scope info — all locals live
// at the function level — so DropElab places drops at the function's
// `Return`. The fixture pins this behavior; a future scope-tree stage
// will move the inner drop to the inner-block exit.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func example() {
    let outer = Handle(fd: 1);
    {
        let inner = Handle(fd: 2);
    }
}
