// test: diagnostics
// stdlib: true
//
// `extend (X, Y): P where X: P, Y: P` makes a tuple conform only when its
// elements meet the where clause. `(A, Int64)` doesn't (`Int64` isn't a
// `FromRequest`), so it is rejected at the call — not accepted by inference
// and left to fail after monomorphization.

module Test

protocol FromRequest {
    static func extract(from raw: Int64) -> Self
}

struct A: FromRequest {
    var v: Int64
    static func extract(from raw: Int64) -> A { A(v: raw) }
}

extend (X, Y): FromRequest where X: FromRequest, Y: FromRequest {
    static func extract(from raw: Int64) -> (X, Y) { (X.extract(from: raw), Y.extract(from: raw)) }
}

func run[I](raw: Int64) -> I where I: FromRequest { I.extract(from: raw) }

func main() -> Int64 {
    let (a, b): (A, Int64) = run(4); // ERROR(E107): (A, Int64) !: FromRequest
    a.v + b
}
