// test: diagnostics
// stdlib: false

module Main

struct Pair[A, B] {
    let first: A
    let second: B
    
    func getSecond() -> B {
        self.second
    }
}
