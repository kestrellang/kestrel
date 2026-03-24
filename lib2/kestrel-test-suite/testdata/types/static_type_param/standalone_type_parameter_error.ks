// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    init()
}
func bad[T]() where T: Factory {
    let x = T; // ERROR:
}
