// test: diagnostics
// stdlib: false

module Test
@dummy
enum Option[T] {
    case Some(value: T)
    case None
}
