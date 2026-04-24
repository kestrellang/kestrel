// test: diagnostics
// stdlib: false

module Test

struct Value {
    private var _data: lang.i64

    var data: lang.i64 {
        get {
            self._data
        }
        set {
            self._data = newValue
        }
    }

    init() {
        self._data = 0
    }
}
