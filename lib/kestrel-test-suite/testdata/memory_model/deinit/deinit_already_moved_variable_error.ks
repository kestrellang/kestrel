// test: diagnostics
// stdlib: false

module Test

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func consume(consuming h: Handle) {}

func example() {
    var handle = Handle(fd: 42);
    consume(handle);
    deinit handle; // ERROR: moved
}
