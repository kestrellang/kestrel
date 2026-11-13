// test: diagnostics
// stdlib: true

// `a + b` is a call to the `Addable` requirement, so it needs
// `typeof a: Addable[typeof b]`. `V` conforms only to `Addable[V]` (the
// default), so `V + Float64` is an error. Inference once accepted it by
// finding any member named `add`, while codegen called the `Addable[V]`
// witness with a Float64 argument (backend verifier: "arg 1 has type f64").

module Test

struct V: Addable {
    var x: Int64
    type Output = Int64
    static var zero: V { V(x: 0) }
    consuming func add(consuming other: V) -> Int64 { 2 }
}

func use() -> Int64 {
    V(x: 1) + 1.5 // ERROR:
}
