// test: diagnostics
// stdlib: false

module Main

struct Box[T] {
    let value: T

    func read() -> T {
        self.value
    }
}

func main() -> lang.i64 {
    let b = Box[lang.i64](value: 42);
    b.read()
}
