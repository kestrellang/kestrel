// test: diagnostics
// stdlib: false

module Main

struct Builder {
    let value: lang.i64

    func add(n: lang.i64) -> Builder {
        Builder(value: lang.i64_add(self.value, n))
    }

    func multiply(n: lang.i64) -> Builder {
        Builder(value: lang.i64_mul(self.value, n))
    }

    func build() -> lang.i64 {
        self.value
    }
}

func main() -> lang.i64 {
    Builder(value: 0)
        .add(5)
        .multiply(3)
        .build()
}
