// test: diagnostics
// stdlib: false

module Main

protocol Lifecycle {
    func start()
    mutating func update()
    consuming func finish() -> lang.i64
}
