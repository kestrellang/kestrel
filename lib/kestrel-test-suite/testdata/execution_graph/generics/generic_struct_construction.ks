// test: diagnostics
// stdlib: false

module Main

struct Box[T] {
    let value: T
}

func makeBox() -> Box[lang.i64] {
    Box[lang.i64](value: 42)
}
