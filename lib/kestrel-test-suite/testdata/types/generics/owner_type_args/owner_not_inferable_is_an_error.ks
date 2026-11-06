// test: diagnostics
// stdlib: true

module Test

// Bare `Box` is `Box[_]`; when nothing determines `T` it is an inference
// error at the call — never a compiler crash. (It reads the same as a generic
// free function whose `T` nothing determines: the implicit `Copyable` bound
// on the unsolved `T` is what reports.)

struct Box[T] {
    static func count() -> Int64 { 1 }
}

func use() -> Int64 {
    Box.count() // ERROR: does not satisfy constraint
}
