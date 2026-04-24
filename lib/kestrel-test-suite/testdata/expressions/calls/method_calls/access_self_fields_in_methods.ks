// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
    var z: lang.i64

    func getX() -> lang.i64 { self.x }
    func getY() -> lang.i64 { self.y }
    mutating func getZ() -> lang.i64 { self.z }
}
