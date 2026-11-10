// test: execution
// stdlib: true
// expect-stdout: area 9\narea 9\nsquare 2\n

// A generic requirement can't sit in an existential's function table, but an
// `@existentialOnly` default written with dispatchable requirements stands in
// for it on `any P`: in direct calls and as the witness that lets `any P` be
// passed to generic code. Concrete conformers keep their own implementation.

module Test

protocol Shape {
    func area() -> Int64
    func describe[W](tag: W) -> String
}

extend Shape {
    @existentialOnly
    func describe[W](tag: W) -> String { "area \(self.area())" }
}

struct Sq: Shape {
    var s: Int64
    func area() -> Int64 { self.s * self.s }
    func describe[W](tag: W) -> String { "square \(self.s)" }
}

func viaGeneric(x: some Shape) -> String { x.describe(0) }

@main
func main() {
    let e: any Shape = Sq(s: 3);
    println(e.describe(1));
    println(viaGeneric(e));
    println(viaGeneric(Sq(s: 2)));
}
