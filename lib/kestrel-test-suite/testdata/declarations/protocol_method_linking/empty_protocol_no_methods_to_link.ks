// test: diagnostics
// stdlib: false
module Test

protocol Marker { }
struct Point: Marker {
    func draw() { }
}
