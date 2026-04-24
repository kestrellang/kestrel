// test: diagnostics
// stdlib: true

module Test

struct Handle: not Copyable {
    var fd: Int64
}

func consume(consuming h: Handle) {}

func test() {
    var h = Handle(fd: 42);
    consume(h);
    consume(h) // ERROR: use of moved value
}
