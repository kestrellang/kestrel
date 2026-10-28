// test: execution
// stdlib: true
// expect-exit: 0

module Test

// Both brackets written: `Pair[Int64].combine[String](..)` — `[Int64]` is the
// type's `A`, `[String]` the method's own `B`.

struct Pair[A] where A: Formattable {
    static func combine[B](a: A, b: B) -> Int64 where B: Formattable { "\(a)\(b)".bytes.count }
}

@main
func main() -> Int64 {
    if Pair[Int64].combine[String](12, "abc") != 5 { return 1 }
    0
}
