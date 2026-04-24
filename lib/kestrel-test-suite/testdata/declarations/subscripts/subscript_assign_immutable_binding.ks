// test: diagnostics
// stdlib: false

module Test

struct Holder {
    var value: lang.i64

    public init(value: lang.i64) {
        self.value = value
    }

    public subscript(index: lang.i64) -> lang.i64 {
        get { self.value }
        set { self.value = newValue }
    }
}

func seed() -> lang.i64 { 10 }
func zero() -> lang.i64 { 0 }

func main() -> lang.i64 {
    let h = Holder(value: seed());
    h(zero()) = seed(); // ERROR: cannot assign to subscript on immutable binding
    0
}
