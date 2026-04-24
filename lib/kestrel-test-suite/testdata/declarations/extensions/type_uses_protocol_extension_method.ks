// test: diagnostics
// stdlib: false
module Test

protocol Drawable {
    func draw()
}
extend Drawable {
    func helperMethod() { }
}
struct Circle: Drawable {
    func draw() { }
}
func test() {
    let c = Circle();
    c.helperMethod();
}
