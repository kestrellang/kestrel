// test: diagnostics
// stdlib: false

module Main

struct Box[T] {
    let value: T
    
    func read() -> T {
        self.value
    }
}
