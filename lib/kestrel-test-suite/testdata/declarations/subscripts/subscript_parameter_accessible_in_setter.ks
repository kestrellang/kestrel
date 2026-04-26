// test: diagnostics
// stdlib: false

module Test

struct Int {}
struct Container[T] {
    private var data1: T
    private var data2: T

    public init(data: T) {
        self.data1 = data;
        self.data2 = data;
    }

    public subscript(dummy: lang.i64) -> T {
        get {
            // Use dummy to ensure it's accessible
            let _unused = dummy;
            self.data1
        }
        set {
            // Use dummy to ensure it's accessible
            let _unused = dummy;
            self.data1 = newValue
        }
    }
}
