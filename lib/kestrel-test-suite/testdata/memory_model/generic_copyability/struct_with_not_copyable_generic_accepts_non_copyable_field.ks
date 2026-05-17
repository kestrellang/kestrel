// test: diagnostics
// stdlib: true

module Test

struct Handle: not Copyable {
    var fd: lang.i64
}

struct Wrapper[T] where T: not Copyable {
    var value: T
}

func test() {
    var h = Handle(fd: 1);
    var w = Wrapper(value: h);
}
