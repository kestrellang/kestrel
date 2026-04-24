// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func getVal(h: Handle) -> lang.i64 {
    return h.fd
}

func example(cond: lang.i1) {
    let handle = Handle(fd: 42);
    if cond {
        let x = getVal(handle);
    } else {
        let y = getVal(handle);
    }
}
