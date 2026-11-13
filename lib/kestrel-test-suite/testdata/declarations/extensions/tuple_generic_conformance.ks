// test: execution
// stdlib: true
// expect-exit: 0
//
// A tuple of conformers conforms structurally: `extend (X, Y): P where X: P, Y: P`.
// Perch needs this for handler inputs like `(Path[Int64], Json[T])`. Today the
// extension's type params aren't bound ("cannot find type 'X' in this scope").

module Test

protocol FromRequest {
    static func extract(from raw: Int64) -> Self
}

struct A: FromRequest {
    var v: Int64
    static func extract(from raw: Int64) -> A { A(v: raw) }
}

struct B: FromRequest {
    var v: Int64
    static func extract(from raw: Int64) -> B { B(v: raw * 10) }
}

extend (X, Y): FromRequest where X: FromRequest, Y: FromRequest {
    static func extract(from raw: Int64) -> (X, Y) { (X.extract(from: raw), Y.extract(from: raw)) }
}

func run[I](raw: Int64) -> I where I: FromRequest { I.extract(from: raw) }

@main
func main() -> Int64 {
    let (a, b): (A, B) = run(4);
    a.v + b.v - 44
}
