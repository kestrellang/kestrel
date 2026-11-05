// test: execution
// stdlib: true

// A builder method returning `&mutating Self`, called as a statement and its
// result discarded, must end that borrow at the end of the statement. Moving
// the value afterwards (here, returning it) is then legal.
//
// Known gap: the borrow stays live to the end of the scope, and moving the
// value fails OSSA verification ("cannot take from ...: active borrow(s)").
// Calling a method on it afterwards works; only the move fails. std.system's
// Command.shell works around it.

module Test

struct Builder: not Copyable {
    var parts: Array[String]

    init(first: String) { self.parts = [first]; }

    mutating func add(part: String) -> &mutating Builder {
        self.parts.append(part);
        self
    }
}

func make() -> Builder {
    var b = Builder("a");
    b.add("b");
    b
}

@main
func main() -> lang.i64 {
    let b = make();
    if b.parts.count != 2 { return 1 }
    0
}
