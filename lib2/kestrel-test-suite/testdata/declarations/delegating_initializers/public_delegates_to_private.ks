// test: diagnostics
// stdlib: false

module Test

struct Secret {
    var data: lang.i64

    private init(data: lang.i64) {
        self.data = data
    }

    public init() {
        self.init(42)
    }
}
