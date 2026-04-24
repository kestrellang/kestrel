// test: diagnostics
// stdlib: false

module Test

struct Int {}
struct Matrix[T] {
    private var data: T

    public init(data: T) {
        self.data = data
    }

    public subscript(row: lang.i64, col: lang.i64) -> T {
        get {
            // Use row and col to ensure they're accessible
            let _unused1 = row;
            let _unused2 = col;
            self.data
        }
    }
}
