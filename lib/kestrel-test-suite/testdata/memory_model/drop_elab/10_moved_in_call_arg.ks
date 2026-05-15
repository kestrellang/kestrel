// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: a non-Copy local is moved into a function call's argument.
// The dataflow tracks the kill from the `Value::Move` in `Call.args`,
// so the local's path is uninit at the return — no drop emitted.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func take(consuming h: Handle) {}

func example() {
    let h = Handle(fd: 1);
    take(h);
}
