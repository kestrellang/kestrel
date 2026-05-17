// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func example() {
    var handle = Handle(fd: 42);
    deinit handle;
    deinit handle; // ERROR: moved
}
