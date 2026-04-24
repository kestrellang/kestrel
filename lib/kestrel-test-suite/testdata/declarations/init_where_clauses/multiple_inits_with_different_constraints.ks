// test: diagnostics
// stdlib: false

module Test

protocol Readable {
    func read() -> lang.i64
}

protocol Writable {
    func write(value: lang.i64)
}

struct Store {
    var data: lang.i64

    init[R](reader reader: R) where R: Readable {
        self.data = reader.read()
    }

    init(value value: lang.i64) {
        self.data = value
    }
}
