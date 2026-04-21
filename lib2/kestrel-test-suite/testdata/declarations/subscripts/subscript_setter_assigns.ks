// test: execution
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
func newVal() -> lang.i64 { 42 }
func zero() -> lang.i64 { 0 }

func main() -> lang.i64 {
    var h = Holder(value: seed());
    if lang.i64_ne(h(zero()), seed()) { return 1 }
    h(zero()) = newVal();
    if lang.i64_ne(h(zero()), newVal()) { return 2 }
    0
}
