// test: diagnostics
// stdlib: false

module Test

protocol Drawable {
    func draw()
}
protocol Shape: Drawable {
    func area() -> lang.i64
}
