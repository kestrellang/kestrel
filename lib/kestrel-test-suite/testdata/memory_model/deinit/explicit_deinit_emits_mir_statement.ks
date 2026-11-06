// test: diagnostics
// stdlib: false

module Test

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func example() {
    let handle = Handle(fd: 42);
    deinit handle;
}
