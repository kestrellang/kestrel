// test: diagnostics
// stdlib: false

module Test

protocol Drawable {
    func draw()
}

struct Shape: not Drawable {} // ERROR: not a language feature protocol
