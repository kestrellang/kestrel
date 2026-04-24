// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
}

func consume(consuming h: Handle) {}
func borrow(h: Handle) {}

func test() {
    var h = Handle(fd: 42);
    consume(h);
    borrow(h); // ERROR: use of moved value
    consume(h)
}
