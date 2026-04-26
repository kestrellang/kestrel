// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func example() {
    let h1 = Handle(fd: 1);
    let h2 = Handle(fd: 2);
}
