// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
}

func consume(consuming h: Handle) -> lang.i64 { h.fd }

func test() -> lang.i64 {
    let h = Handle(fd: 42);
    consume(h)
}
