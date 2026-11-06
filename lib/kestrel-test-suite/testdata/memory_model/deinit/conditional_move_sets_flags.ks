// test: diagnostics
// stdlib: false

module Test

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func consume(consuming h: Handle) {}

func example(cond: lang.i1) {
    let handle = Handle(fd: 42);
    if cond {
        consume(handle);
    } else {
    }
}
