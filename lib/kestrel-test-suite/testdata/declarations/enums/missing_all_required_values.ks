// test: diagnostics
// stdlib: false

module Test

enum Point {
    case Location(x: lang.i64, y: lang.i64)
}

func test() -> Point {
    Point.Location() // ERROR: no matching overload
}
