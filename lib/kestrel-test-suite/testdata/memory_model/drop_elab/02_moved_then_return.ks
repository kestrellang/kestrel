// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: local moved into a consuming call, then return.
//
// `h` is moved out before the return, so its move-path is uninit at
// the terminator. DropElab should *not* emit a drop for `h` — the
// dataflow proves the path is dead.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func consume(consuming h: Handle) {}

func example() {
    let h = Handle(fd: 1);
    consume(h);
}
