// test: diagnostics
// stdlib: true

module Test

struct Handle: not Copyable {
    var fd: lang.i64
}

func process[T](consuming x: T) where T: not Copyable { }

func test() {
    var h = Handle(fd: 1);
    process(h);  // This should work
}
