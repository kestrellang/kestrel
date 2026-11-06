// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A closure's un-annotated param is borrowed: indirect calls pass it
// `@borrow` and release it themselves. Consuming it in the body (`it + "b"`)
// must copy, not take the caller's value (that was a double free).

module Test

func wrap(f: (String) -> String) -> String {
    let r = f("a");
    r
}

func suffixed(consuming s: String) -> String { s + "!" }

@main
func main() -> lang.i64 {
    if wrap({ it + "b" }).isEqual(to: "ab") == false { return 1 }
    let g: escaping () -> String = { wrap({ (s) in s + "c" }) };
    if g().isEqual(to: "ac") == false { return 2 }
    if wrap(suffixed).isEqual(to: "a!") == false { return 3 }
    0
}
