// test: diagnostics
// stdlib: false

module Test

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
    let x = getId(handle); // ERROR: moved
}
