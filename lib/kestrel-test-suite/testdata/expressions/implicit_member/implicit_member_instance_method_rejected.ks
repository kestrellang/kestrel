// test: diagnostics

// `.name` has no receiver, so it can only name a static member, an enum case,
// or an initializer — not an instance method (which used to be called with
// no `self`, an internal compiler error).

module Test

func g(x: Float64) -> Float64 { x }

func use() -> Float64 {
    g(.round()) // ERROR: instance method 'round' cannot be called on a type
}
