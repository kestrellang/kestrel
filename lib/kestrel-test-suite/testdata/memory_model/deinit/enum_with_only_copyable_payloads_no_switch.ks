// test: diagnostics
// stdlib: false

module Test
@builtin(.Copyable)
protocol Copyable {}

enum Value {
    case Int(val: lang.i64)
    case pair(a: lang.i64, b: lang.i64)
    case none
}

func example() {
    let v = Value.Int(val: 42);
}
