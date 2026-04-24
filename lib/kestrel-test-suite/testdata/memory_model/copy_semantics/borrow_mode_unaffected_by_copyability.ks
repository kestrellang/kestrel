// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
}

func borrow_it(h: Handle) {}

func test() {
    let handle = Handle(fd: 42);
    borrow_it(handle)
}
