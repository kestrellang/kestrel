// test: diagnostics
// stdlib: false

module Test

struct Wrapper[T] {
    var value: T
    var label: lang.str

    init(value: T, label: lang.str) {
        self.value = value;
        self.label = label
    }

    init(value: T) {
        self.init(value, "default")
    }
}
