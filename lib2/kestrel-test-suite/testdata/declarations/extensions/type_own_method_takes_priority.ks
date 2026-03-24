// test: diagnostics
// stdlib: false
module Test

protocol Drawable {
    func draw()
}
extend Drawable {
    func helper() { }
}
struct Circle: Drawable {
    func draw() { }
    func helper() { }
}
func test() {
    let c = Circle();
    c.helper();
}
