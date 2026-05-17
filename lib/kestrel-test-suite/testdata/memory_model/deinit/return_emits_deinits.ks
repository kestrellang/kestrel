// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func example() -> lang.i64 {
    let handle = Handle(fd: 42);
    return 0;
}
