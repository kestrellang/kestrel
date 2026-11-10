// test: diagnostics
// stdlib: true

// An `@existentialOnly` default serves `any P` alone: a concrete conformer
// that leaves the requirement out is still missing it.

module Test

protocol Shape {
    func area() -> Int64
    func describe[W](tag: W) -> String
}

extend Shape {
    @existentialOnly
    func describe[W](tag: W) -> String { "area \(self.area())" }
}

struct Sq: Shape { // ERROR: type 'Sq' does not implement method 'describe' from protocol 'Shape'
    var s: Int64
    func area() -> Int64 { self.s * self.s }
}
