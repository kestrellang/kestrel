// test: diagnostics
// stdlib: false
module Test

struct Box[T] { var value: T }
extend Box[T] {
    func getValue() -> T { return self.value; }
    mutating func setValue(newValue: T) { self.value = newValue; }
}
func test() -> lang.i64 {
    var b = Box[lang.i64](value: 10);
    b.setValue(20);
    return b.getValue();
}
