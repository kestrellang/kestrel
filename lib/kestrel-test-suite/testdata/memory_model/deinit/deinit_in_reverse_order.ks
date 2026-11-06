// test: diagnostics
// stdlib: false

module Test

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func example() {
    let h1 = Handle(fd: 1);
    let h2 = Handle(fd: 2);
}
