// test: execution
// stdlib: true
// Regression test: extension subscript setter on Array[CustomStruct].
// Ensures the Slice extension subscript set path works for aggregate types.

module Test

struct Point: Cloneable {
    var x: Int64
    var y: Int64

    func clone() -> Point {
        Point(x: self.x, y: self.y)
    }
}

func main() -> lang.i64 {
    var pts = Array[Point](repeating: Point(x: 0, y: 0), count: 3);

    pts(0) = Point(x: 10, y: 20);
    pts(unchecked: 1) = Point(x: 30, y: 40);
    pts(2) = Point(x: 50, y: 60);

    if pts(unchecked: 0).x != 10 { return 1 }
    if pts(unchecked: 0).y != 20 { return 2 }
    if pts(unchecked: 1).x != 30 { return 3 }
    if pts(unchecked: 1).y != 40 { return 4 }
    if pts(unchecked: 2).x != 50 { return 5 }
    if pts(unchecked: 2).y != 60 { return 6 }

    0
}
