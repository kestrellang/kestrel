// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
}

func consume(consuming h: Handle) {}

func test() -> lang.i64 {
    var h = Handle(fd: 42);
    consume(h);
    h.fd // ERROR: use of moved value
}
