// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Point {
    var x: lang.i64
    var y: lang.i64
}
struct Handle: not Copyable {
    var fd: lang.i64
}

func mixed(consuming p: Point, consuming h: Handle) {}

func test() {
    let pt = Point(x: 1, y: 2);
    let handle = Handle(fd: 42);
    mixed(pt, handle)
}
