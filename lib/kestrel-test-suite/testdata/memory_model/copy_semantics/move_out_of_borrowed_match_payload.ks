// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
}

enum Opt: not Copyable {
    case value(h: Handle)
    case empty
}

func consume(consuming h: Handle) {}

// `o` is borrowed (a plain parameter), so its non-Copyable payload cannot be
// moved out of the match binding into a consuming call. It needs `consuming o`.
func example(o: Opt) {
    match o {
        .value(h: payload) => consume(payload), // ERROR: borrowed
        .empty => {}
    }
}
