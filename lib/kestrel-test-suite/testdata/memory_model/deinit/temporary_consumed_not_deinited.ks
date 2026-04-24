// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func makeHandle() -> Handle {
    return Handle(fd: 42);
}

func consume(consuming h: Handle) {}

func example() {
    consume(makeHandle());
}
