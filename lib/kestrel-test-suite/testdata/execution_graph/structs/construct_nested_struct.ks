// test: diagnostics
// stdlib: false

module Main

struct Inner {
    let value: lang.i64
}

struct Outer {
    let inner: Inner
}

func main() -> Outer {
    let i = Inner(value: 42);
    Outer(inner: i)
}
