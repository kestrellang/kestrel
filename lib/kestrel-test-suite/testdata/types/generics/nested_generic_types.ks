// test: diagnostics
// stdlib: false

module Test

struct Box[T] { var value: T }
struct Pair[A, B] { var first: A; var second: B }
struct Nested { var box: Box[Pair[lang.i64, lang.str]] }
