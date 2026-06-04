// test: execution
// stdlib: true
// expect-exit: 0

module Test

protocol Summable {
    func value() -> Int64
}

// Extension method — default implementation on the protocol
extend Summable {
    func doubled() -> Int64 {
        return self.value() + self.value()
    }
}

struct MyVal {
    var n: Int64
}

extend MyVal: Summable {
    func value() -> Int64 { return self.n }
}

// Calls the extension method through the witness table
func getDoubled[T](s: T) -> Int64 where T: Summable {
    return s.doubled()
}

@main
func main() -> lang.i64 {
    let v = MyVal(n: 21);
    let result = getDoubled(v);
    if result != 42 { return 1 }
    return 0
}
