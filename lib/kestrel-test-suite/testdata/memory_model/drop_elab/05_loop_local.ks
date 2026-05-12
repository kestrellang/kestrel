// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: a non-Copy local declared inside a `loop` body.
//
// MIR move-paths are root-only at Stage 7, so `h` shares one path with
// itself across loop iterations. The dataflow joins over the back-edge
// and either marks the path uninit (if all iterations move it out) or
// keeps it MaybeInit (if some don't). The drop is emitted at the
// function-exit `Return`. A future "scope-tree drops" stage will move
// it to the loop-body's scope edge.

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
    loop {
        let h = Handle(fd: 1);
        consume(h);
        break;
    }
}
