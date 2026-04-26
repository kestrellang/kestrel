// test: diagnostics
// stdlib: false
module Test
protocol Drawable {
    func draw()
}
protocol Clickable {
    func onClick()
}
struct Button: Drawable, Clickable { // ERROR: does not implement method 'onClick'
    func draw() { }
}
