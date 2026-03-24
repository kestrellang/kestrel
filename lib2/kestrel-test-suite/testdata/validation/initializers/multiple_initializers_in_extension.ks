// test: diagnostics
// stdlib: false

module Main

public struct Point {
    var x: lang.i64
    var y: lang.i64
}

extend Point {
    public init(x: lang.i64, y: lang.i64) {
        self.x = x;
        self.y = y;
    }

    public init(value: lang.i64) {
        self.x = value;
        self.y = value;
    }
}

public func test() {
    let p1 = Point(1, 2);
    let p2 = Point(5);
}
