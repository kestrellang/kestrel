// test: diagnostics
// stdlib: true

// `extend Tagged` means `Tagged[Int64]`, so a `Tagged[Bool]` does not get
// its members.
module Test

struct Tagged[T = Int64] {
    var value: T
}

extend Tagged {
    func hello() -> Int64 { 1 }
}

func f(t: Tagged[Bool]) -> Int64 {
    t.hello() // ERROR: no member 'hello'
}
