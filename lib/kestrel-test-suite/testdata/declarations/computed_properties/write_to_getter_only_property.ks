// test: diagnostics
// stdlib: false

module Test

struct ReadOnly {
    private var _data: lang.i64

    var data: lang.i64 {
        get { self._data }
    }

    init() { self._data = 0 }
}

func test() {
    var r = ReadOnly();
    r.data = 42; // ERROR: cannot assign
}
