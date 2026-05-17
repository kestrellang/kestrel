// test: diagnostics
// stdlib: false

module Main

struct Container {
    var value: lang.i64

    init(value: lang.i64) {
        self.value = value;
    }

    subscript(index: lang.i64 = 0) -> lang.i64 {
        get { value }
    }
}

func test() -> lang.i64 {
    let c = Container(42);
    c()
}
