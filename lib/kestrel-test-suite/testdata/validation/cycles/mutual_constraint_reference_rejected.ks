// test: diagnostics
// stdlib: false

module Main

protocol Container[T] {
    func read() -> T
}

func swap[T, U](a: T, b: U) -> () where T: Container[U], U: Container[T] { // ERROR: circular generic constraint
    ()
}
