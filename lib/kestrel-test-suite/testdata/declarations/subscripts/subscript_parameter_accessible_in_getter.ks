// test: diagnostics
// stdlib: false

module Test

struct Int {}
struct Container[T] {
    private var data: T

    public init(data: T) {
        self.data = data
    }

    public subscript(dummy: lang.i64) -> T {
        get {
            // Use dummy to ensure it's accessible
            let _unused = dummy;
            self.data
        }
    }
}
