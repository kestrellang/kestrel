// test: diagnostics
// stdlib: false

module Main

protocol Printable {
    func print() -> lang.str
}

protocol Comparable {
    func compare() -> lang.i64
}

func process[T, U](a: T, b: U) -> () where T: Printable, U: Comparable {
    ()
}
