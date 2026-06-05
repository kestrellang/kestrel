// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

enum R[T, E]: not Copyable {
    case Ok(T)
    case Err(E)
}

extend R[T, E]: Copyable where T: Copyable, E: Copyable { }

struct Plain {
    var x: lang.i64
}

func needsCopyable[U](x: U) where U: Copyable { }

// R[Plain, Plain] — both args Copyable, so R[Plain, Plain] must be Copyable.
// No diagnostic expected.
func main() {
    let r: R[Plain, Plain] = .Ok(Plain(x: 1));
    needsCopyable(r);
}
