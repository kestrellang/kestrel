// test: diagnostics
// stdlib: false
module Test

protocol Drawable {
    func draw()
}
extend Drawable {
    func helper() { }
}
