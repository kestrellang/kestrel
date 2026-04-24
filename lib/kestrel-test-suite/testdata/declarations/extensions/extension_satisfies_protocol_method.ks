// test: diagnostics
// stdlib: false
module Test
protocol Hashable { func hash() -> lang.i64 }
struct Point { var x: lang.i64; var y: lang.i64 }
extend Point: Hashable {
    func hash() -> lang.i64 { return lang.i64_add(self.x, self.y); }
}
func getHash[T](value: T) -> lang.i64 where T: Hashable { return value.hash(); }
func test() -> lang.i64 {
    let p = Point(x: 3, y: 4);
    return getHash(p);
}
