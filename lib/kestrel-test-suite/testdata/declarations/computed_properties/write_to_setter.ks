// test: diagnostics
// stdlib: false

module Test

struct Wrapper {
    var value: lang.i64

    mutating func setValue(newValue: lang.i64) {
        self.value = newValue
    }

    init() { self.value = 0 }
}

func test() {
    var w = Wrapper();
    w.setValue(42);
}
