// test: diagnostics
// stdlib: false

module Test

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
