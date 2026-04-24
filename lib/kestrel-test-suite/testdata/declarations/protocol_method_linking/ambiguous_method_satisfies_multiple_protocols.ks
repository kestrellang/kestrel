// test: diagnostics
// stdlib: false
module Test

protocol Drawable {
    func render()
}
protocol Paintable {
    func render()
}
struct Canvas: Drawable, Paintable {
    func render() { } // ERROR: ambiguous
}
