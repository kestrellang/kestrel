// test: diagnostics
// stdlib: false
module Test

protocol Drawable {
    func draw()
}
struct Circle: Drawable {
    static func draw() { } // ERROR: receiver
}
