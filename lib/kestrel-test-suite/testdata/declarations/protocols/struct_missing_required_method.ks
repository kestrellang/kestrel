// test: diagnostics
// stdlib: false
module Test
protocol Drawable {
    func draw()
}
struct Circle: Drawable { } // ERROR: does not implement method 'draw'
