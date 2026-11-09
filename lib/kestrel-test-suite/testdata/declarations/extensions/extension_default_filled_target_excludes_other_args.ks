// test: diagnostics
// stdlib: true

// `extend Pair[T]` means `Pair[T, Int64]`, so a `Pair[T, Bool]` does not get
// its members.
module Test

struct Pair[T, U = Int64] {
    var first: T
    var second: U
}

extend Pair[T] {
    func hello() -> Int64 { 1 }
}

func f(p: Pair[Bool, Bool]) -> Int64 {
    p.hello() // ERROR: no member 'hello'
}
