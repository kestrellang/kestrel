// test: execution
// stdlib: true
// expect-exit: 0

module Test

// Bare owner and an own method param, both inferred from the arguments.

struct Pair[A] where A: Formattable {
    static func combine[B](a: A, b: B) -> Int64 where B: Formattable { "\(a)\(b)".bytes.count }
}

@main
func main() -> Int64 {
    if Pair.combine(12, "abc") != 5 { return 1 }
    0
}
