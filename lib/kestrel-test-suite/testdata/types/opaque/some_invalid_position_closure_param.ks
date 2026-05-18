// test: diagnostics
// stdlib: false

// `some P` in the parameter position of a returned function type.
// The parser accepts it but type checking rejects the function type
// because the opaque bound produces a conformance error.
// TODO: implement E466 for a clearer diagnostic.

module Test

protocol Shape {
    func area() -> lang.i64
}

func bad() -> (some Shape) -> lang.i64 {
    1 // ERROR: does not conform
}
