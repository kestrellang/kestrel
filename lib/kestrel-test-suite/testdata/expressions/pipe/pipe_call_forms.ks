// test: execution
// stdlib: true
// expect-exit: 0

// The right side can be any call form: method, static, initializer,
// subscript, and a bare method or static path.

module Test

struct Point {
    var x: Int64
    var y: Int64

    func sum() -> Int64 { self.x + self.y }
    func scaled(by k: Int64) -> Int64 { (self.x + self.y) * k }
    static func origin(offset: Int64) -> Point { Point(x: offset, y: offset) }
}

struct Adder {
    let base: Int64
    func add(x: Int64) -> Int64 { self.base + x }
}

@main
func main() -> lang.i64 {
    let a = Adder(base: 10);
    // method with a slot
    if (5 |> a.add(_)) != 15 { return 1 }
    // bare method path: `x |> a.add` is `a.add(x)`
    if (5 |> a.add) != 15 { return 2 }
    // initializer with a slot
    let p = 3 |> Point(x: _, y: 4);
    if p.x != 3 or p.y != 4 { return 3 }
    // static method, slot and bare path
    if (2 |> Point.origin(_)).sum() != 4 { return 4 }
    if (2 |> Point.origin).sum() != 4 { return 5 }
    // the piped value as a receiver's argument
    if (3 |> p.scaled(by: _)) != 21 { return 6 }
    // subscript (call syntax) with a slot
    let xs = [10, 20, 30];
    if (1 |> xs(_)) != 20 { return 7 }
    0
}
