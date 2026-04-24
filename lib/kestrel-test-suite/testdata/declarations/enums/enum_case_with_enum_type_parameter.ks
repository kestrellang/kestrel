// test: diagnostics
// stdlib: false
module Test
enum Inner {
    case Value
}

enum Outer {
    case Contains(inner: Inner)
}
