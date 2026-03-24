// test: diagnostics
// stdlib: false

module Main

struct Pair[A, B] {
    let first: A
    let second: B
    
    func getFirst() -> A {
        self.first
    }
}
