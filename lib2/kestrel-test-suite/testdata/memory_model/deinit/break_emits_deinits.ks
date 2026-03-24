// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func example() {
    loop {
        let h = Handle(fd: 1);
        break;
    }
}
