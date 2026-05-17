// test: diagnostics
// stdlib: false

module Test

protocol Base {
    init()
}
protocol Left: Base {}
protocol Right: Base {}
func make[T]() -> T where T: Left, T: Right {
    return T()
}
