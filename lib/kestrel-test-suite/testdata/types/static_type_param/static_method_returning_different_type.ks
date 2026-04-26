// test: diagnostics
// stdlib: false

module Test

protocol Counter {
    static func count() -> lang.i64
}
func getCount[T]() -> lang.i64 where T: Counter {
    return T.count()
}
