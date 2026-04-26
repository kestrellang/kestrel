// test: diagnostics
// stdlib: false

module Test

struct Inner {
    let value: lang.i64

    deinit {
        // Inner deinit
    }
}

struct Outer {
    let first: Inner
    let second: Inner

    deinit {
        // Outer deinit runs first, then fields in reverse order
    }
}

func main() -> lang.i64 {
    let o = Outer(first: Inner(value: 1), second: Inner(value: 2));
    lang.i64_add(o.first.value, o.second.value)
}
