// test: diagnostics
// stdlib: false

module Test

struct Int {}
struct Container[T] {
    private var data: T

    public init(data: T) {
        self.data = data
    }

    public subscript(index: lang.i64) -> T {
        get {
            self.data
        }
        set {
            self.data = newValue
        }
    }
}
