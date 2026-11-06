// test: diagnostics
// stdlib: false

module Test

// The rule holds for generic enums too.

enum Opt[T] {
    case Some(T)
    case None
    static var made: lang.i64 = 0; // ERROR: static stored properties not supported in generic types
}
