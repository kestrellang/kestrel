// test: diagnostics
// stdlib: false

module Test

struct Box[T] { var value: T }
func makeBox[T](value: T) -> Box[T] { Box(value: value) }
