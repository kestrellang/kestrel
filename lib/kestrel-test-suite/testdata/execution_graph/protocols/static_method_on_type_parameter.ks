// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    static func create() -> Self
}

func make[T]() -> T where T: Factory {
    return T.create()
}
