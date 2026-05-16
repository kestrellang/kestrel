// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: a `match` arm binds an enum payload and moves it into a
// consuming call.
//
// At Stage 7 the payload binding is its own local. After the consuming
// call, the binding's path is uninit at the arm's join with the
// `Default` arm. The dataflow joins both arms; the resulting state
// determines whether the binding gets `Drop`, `DropIf`, or no drop.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

enum Opt: not Copyable {
    case value(h: Handle)
    case empty
}

func consume(consuming h: Handle) {}

func example(o: Opt) {
    match o {
        .value(h: payload) => consume(payload),
        .empty => {}
    }
}
