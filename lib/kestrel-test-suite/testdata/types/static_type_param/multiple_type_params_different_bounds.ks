// test: diagnostics
// stdlib: false

module Test

protocol FactoryA {
    init()
}
protocol FactoryB {
    init()
}
func makeBoth[A, B]() -> (A, B) where A: FactoryA, B: FactoryB {
    return (A(), B())
}
