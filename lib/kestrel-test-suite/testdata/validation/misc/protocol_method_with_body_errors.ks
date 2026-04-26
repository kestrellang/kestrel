// test: diagnostics
// stdlib: false

module Test
protocol Drawable {
    func draw() -> () { } // ERROR: cannot have a body
}
