// test: diagnostics
// stdlib: false

module Test

struct Box[T] { var value: T }
func unbox[T](box: Box[T]) -> T { box.value }
