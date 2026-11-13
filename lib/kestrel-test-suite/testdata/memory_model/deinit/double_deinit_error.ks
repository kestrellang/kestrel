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
    var handle = Handle(fd: 42);
    deinit handle;
    deinit handle; // ERROR: moved
}
