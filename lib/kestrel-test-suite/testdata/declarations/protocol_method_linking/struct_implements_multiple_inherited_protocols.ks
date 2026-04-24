// test: diagnostics
// stdlib: false
module Test

protocol Drawable {
    func draw()
}
protocol Clickable {
    func onClick()
}
protocol Widget: Drawable, Clickable {
    func update()
}
struct Button: Drawable, Clickable, Widget {
    func draw() { }
    func onClick() { }
    func update() { }
}
