// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    init()
}

func make[T]() -> T where T: Factory {
    return T()
}
