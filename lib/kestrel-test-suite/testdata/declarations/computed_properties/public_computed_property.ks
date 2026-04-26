// test: diagnostics
// stdlib: false

module Test

struct Data {
    private var _value: lang.i64

    public var value: lang.i64 {
        get { self._value }
        set { self._value = newValue }
    }

    init() { self._value = 0 }
}
