// test: diagnostics
// stdlib: false

module Test

protocol Source {
    func value() -> lang.i64
}

struct Private {
    var data: lang.i64

    private init[T](source: T) where T: Source {
        self.data = source.value()
    }

    static func create[T](source: T) -> Private where T: Source {
        Private(source)
    }
}
