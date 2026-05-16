// test: mir
// stdlib: false
// mir-filter: Test.consume

// Scenario: a `consuming` parameter (owned at MIR — its type is `Handle`,
// not `Ref(Handle)`/`RefMut(Handle)`). The body moves the parameter into
// a second consuming call, so the original-parameter path is uninit at
// return.
//
// At Stage 7 DropElab skips parameters entirely, so no drop is emitted
// for `h`. The fixture pins that behavior. A future "drop owned params"
// stage will flip this to emit a `DropIf %h if _init_h`.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func sink(consuming h: Handle) {}

func consume(consuming h: Handle) {
    sink(h);
}
