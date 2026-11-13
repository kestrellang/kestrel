// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

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
