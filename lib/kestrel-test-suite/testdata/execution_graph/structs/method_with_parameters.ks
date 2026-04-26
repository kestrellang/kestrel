// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
    
    func add(dx: lang.i64, dy: lang.i64) -> Point {
        Point(x: lang.i64_add(self.x, dx), y: lang.i64_add(self.y, dy))
    }
}
