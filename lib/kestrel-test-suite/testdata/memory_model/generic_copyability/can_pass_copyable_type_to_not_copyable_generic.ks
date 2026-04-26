// test: diagnostics
// stdlib: true

module Test

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func process[T](consuming x: T) where T: not Copyable { }

func test() {
    var p = Point(x: 1, y: 2);
    process(p);  // Copyable types work too
}
