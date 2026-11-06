// test: execution
// stdlib: true
// expect-exit: 0

module Test

// The static method returns a non-`Box` type, so `T` can only come from the
// argument — nothing downstream (result type) can recover it.

struct Box[T] {
    static func count(v: T, n: Int64) -> Int64 { n + 1 }
}

@main
func main() -> Int64 {
    if Box.count("x", 4) != 5 { return 1 }
    if Box[String].count("y", 6) != 7 { return 2 }
    0
}
