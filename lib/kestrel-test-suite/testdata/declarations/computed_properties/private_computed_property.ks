// test: diagnostics
// stdlib: false

module Test

struct Internal {
    private var _data: lang.i64

    private var data: lang.i64 {
        get { self._data }
        set { self._data = newValue }
    }

    init() { self._data = 0 }

    func useData() -> lang.i64 {
        self.data
    }
}
