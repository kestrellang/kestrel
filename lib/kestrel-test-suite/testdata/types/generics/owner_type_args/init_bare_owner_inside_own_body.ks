// test: execution
// stdlib: true
// expect-exit: 0

module Test

// Inside `Box[T]`, bare `Box(value: s)` is `Box[_]` — here `Box[String]`, a
// different instantiation than the enclosing `Box[T]` — and `Self(value:)` is
// `Box[T]`.

struct Box[T] {
    var value: T
    func label(s: String) -> Box[String] { Box(value: s) }
    func copy() -> Box[T] { Self(value: self.value) }
}

@main
func main() -> Int64 {
    let b = Box[Int64](value: 5);
    if b.label("hi").value != "hi" { return 1 }
    if b.copy().value != 5 { return 2 }
    0
}
