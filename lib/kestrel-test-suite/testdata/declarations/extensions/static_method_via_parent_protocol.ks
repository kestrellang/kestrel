// test: execution
// stdlib: false
// expect-exit: 0

// `A.greet()` resolves to a static method declared in `extend Base`,
// where `A: Child` and `Child: Base`. The lookup walks `ConformingProtocols`
// transitively so an indirect (parent-protocol) extension is reachable
// via any descendant conformer.

module Main

protocol Base {}

extend Base {
    public static func greet() -> lang.i64 { 100 }
}

protocol Child: Base {}

struct A {}

extend A: Child {}

func main() -> lang.i64 {
    lang.i64_sub(A.greet(), 100)
}
