// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    init()
    static func create() -> Self
}
func makeBothWays[T]() -> (T, T) where T: Factory {
    return (T(), T.create())
}
