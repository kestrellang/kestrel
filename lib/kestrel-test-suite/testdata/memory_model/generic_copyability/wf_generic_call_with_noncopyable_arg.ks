// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

struct Resource: not Copyable {
    var fd: lang.i64
}

// Calling a Copyable-by-default generic function with a non-Copyable arg.
func duplicate[T](x: T) -> T { return x; }

func consume(r: Resource) {
    let y = duplicate(r); // ERROR: !: Copyable
}
