// test: diagnostics
// stdlib: false

module Test

protocol Drawable {
    func draw()
}

struct Circle: Drawable {
    func draw() { }
}
