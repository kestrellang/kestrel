// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

enum Resource: not Copyable {
    case file(handle: Handle)
    case none
}

func example() {
    let r = Resource.file(handle: Handle(fd: 42));
}
