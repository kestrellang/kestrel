// test: diagnostics
// stdlib: false

module Test

protocol Convertible {
    func convert() -> lang.i64
}

struct Public {
    var data: lang.i64

    public init[T](from: T) where T: Convertible {
        self.data = from.convert()
    }
}
