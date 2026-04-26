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

func useRef(handle h: Handle) -> lang.i64 {
    return h.fd
}

func example() -> lang.i64 {
    let result = useRef(handle: makeHandle());
    return result;
}
