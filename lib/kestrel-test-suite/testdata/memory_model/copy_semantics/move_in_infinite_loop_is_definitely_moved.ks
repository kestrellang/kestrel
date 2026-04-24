// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
}

func consume(consuming h: Handle) {}

func test() {
    var h = Handle(fd: 42);
    loop {
        consume(h);
        break
    }
    consume(h) // ERROR: use of moved value
}
