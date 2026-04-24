// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func example(cond: lang.i1) {
    if cond {
        let h1 = Handle(fd: 1);
    } else {
        let h2 = Handle(fd: 2);
    }
}
