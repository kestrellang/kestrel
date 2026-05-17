// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
}

func consume(consuming h: Handle) {}

func test(cond: lang.i1) {
    var h = Handle(fd: 42);
    if cond {
        consume(h)
    }
    consume(h) // ERROR: may have been moved
}
