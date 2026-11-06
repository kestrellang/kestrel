// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `Box(value: "x")`: bare `Box` is `Box[_]`, inferred from the argument.

struct Box[T] { var value: T }

@main
func main() -> Int64 {
    let b = Box(value: "x");
    if b.value != "x" { return 1 }
    0
}
