// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func getId(h: Handle) -> lang.i64 {
    return h.fd
}

func example() {
    var handle = Handle(fd: 42);
    deinit handle;
    let x = getId(h: handle); // ERROR: moved
}
