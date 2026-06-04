// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: early return with a live non-Copy local.
//
// `h` is constructed and never moved; the function returns immediately
// after. DropElab should emit one unconditional `drop %h` before `return`.

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
    return;
}
