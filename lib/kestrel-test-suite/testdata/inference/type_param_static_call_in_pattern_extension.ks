// test: execution
// stdlib: true
// expect-exit: 3

// `A.make()` on a type param bound by a target-arg pattern (`Path[(A, B)]`)
// is a static call, not a generic call with slots to infer. Naming every
// Def's type-arg slots for E105 once reported these as "cannot infer a type
// parameter of this call" (perch's `extend Path[(A, B)]: FromRequest`).

module Test

protocol Make {
    static func make() -> Self
}

extend Int64: Make {
    static func make() -> Int64 { 3 }
}

struct Path[T] {
    var v: T
}

protocol FromParts {
    static func fromParts() -> Self
}

extend Path[(A, B)]: FromParts where A: Make, B: Make {
    static func fromParts() -> Path[(A, B)] {
        let a = A.make();
        let b = B.make();
        Path(v: (a, b))
    }
}

@main
func main() -> Int64 {
    let p: Path[(Int64, Int64)] = Path.fromParts();
    p.v.0
}
