// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
}

enum Result {
    case Ok(value: Handle)
    case Err(code: lang.i64)
}
