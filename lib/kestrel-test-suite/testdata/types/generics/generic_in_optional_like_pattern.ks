// test: diagnostics
// stdlib: false

module Test

struct Option[T] {
    let value: T
}
type OptionalInt = Option[lang.i64];
type OptionalOptional = Option[Option[lang.str]];
