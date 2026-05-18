// test: execution
// stdlib: false
// expect-exit: 0

// `Self(...)` inside an `extend` body on a concrete type resolves to a
// constructor call on the extended type.

module Main

struct Point {
    public let x: lang.i64
    public let y: lang.i64
    public init(x x: lang.i64, y y: lang.i64) {
        self.x = x;
        self.y = y;
    }
}

extend Point {
    public static func origin() -> Self {
        Self(x: 0, y: 0)
    }
    public static func at(x x: lang.i64, y y: lang.i64) -> Self {
        Self(x: x, y: y)
    }
}

func main() -> lang.i64 {
    let p = Point.at(x: 7, y: 3);
    let o = Point.origin();
    lang.i64_sub(lang.i64_add(p.x, p.y), lang.i64_add(o.x, lang.i64_add(o.y, 10)))
}
