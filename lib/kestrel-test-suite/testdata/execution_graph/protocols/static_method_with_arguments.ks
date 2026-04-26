// test: diagnostics
// stdlib: false

module Test

protocol Convertible {
    static func fromInt(value: lang.i64) -> Self
}

func convert[T](n: lang.i64) -> T where T: Convertible {
    return T.fromInt(n)
}
