// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
    
    static func origin() -> Point {
        Point(x: 0, y: 0)
    }
}

func main() -> Point {
    Point.origin()
}
