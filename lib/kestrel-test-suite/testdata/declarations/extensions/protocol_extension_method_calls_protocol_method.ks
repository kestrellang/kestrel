// test: diagnostics
// stdlib: false
module Test

protocol Drawable {
    func draw()
    func clear()
}
extend Drawable {
    func redraw() {
        self.clear();
        self.draw();
    }
}
struct Circle: Drawable {
    func draw() { }
    func clear() { }
}
func test() {
    let c = Circle();
    c.redraw();
}
