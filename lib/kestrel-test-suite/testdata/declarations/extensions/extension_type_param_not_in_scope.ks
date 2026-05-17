// test: diagnostics
// stdlib: false
module Test

struct Box[T] { var value: T }
extend Box[T] {
    func withU() -> U { return self.value; } // ERROR: U
}
