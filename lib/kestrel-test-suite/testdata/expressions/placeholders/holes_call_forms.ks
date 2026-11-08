// test: execution
// stdlib: true
// expect-exit: 0

// Holes work on every call form: method, static, initializer, subscript,
// and an enum case.

module Test

struct Point {
    var x: Int64
    var y: Int64

    func shifted(by d: Int64) -> Point { Point(x: self.x + d, y: self.y + d) }
    static func diag(n: Int64) -> Point { Point(x: n, y: n) }
}

enum Shape {
    case Square(side: Int64)
}

@main
func main() -> lang.i64 {
    let p = Point(x: 1, y: 2);
    // method
    let shift = p.shifted(by: _);
    if shift(10).x != 11 { return 1 }
    // static
    let diag = Point.diag(_);
    if diag(4).y != 4 { return 2 }
    // initializer, two holes
    let make = Point(x: _, y: _);
    let q = make(5, 6);
    if q.x != 5 or q.y != 6 { return 3 }
    // subscript (call syntax)
    let xs = [10, 20, 30];
    let at = xs(_);
    if at(2) != 30 { return 4 }
    // enum case
    let sq: (Int64) -> Shape = Shape.Square(side: _);
    match sq(3) {
        .Square(side) => if side != 3 { return 5 },
    }
    0
}
