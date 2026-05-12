// test: mir
// stdlib: false
// mir-filter: Test.touch

// Scenario: a `mutating` parameter is encoded as `RefMut(Handle)` at
// MIR — the callee borrows, the caller owns. DropElab skips
// parameters, so no drop is emitted for `h`. The function body also
// has no other locals; the MIR should be drop-free.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func touch(mutating h: Handle) {
    h.fd = 42;
}
