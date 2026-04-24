// test: diagnostics
// stdlib: false
module Test
protocol Drawable {
    func draw()
}
protocol Shape: Drawable {
    func area() -> lang.i64
}
struct Circle: Shape { // ERROR: does not implement method 'draw'
    func area() -> lang.i64 { }
}
