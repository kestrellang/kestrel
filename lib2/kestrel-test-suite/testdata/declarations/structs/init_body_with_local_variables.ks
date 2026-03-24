// test: diagnostics
// stdlib: false
module Test
struct Point {
    var x: lang.i64
    var y: lang.i64

    init(value: lang.i64) {
        let doubled: lang.i64 = value;
        self.x = doubled;
        self.y = doubled;
    }
}
