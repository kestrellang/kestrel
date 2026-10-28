// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Matching through a reference binding (`let r = &e; match r { ... }`) to a
// non-copyable enum reads through the view, like matching a borrowed
// parameter: arm bindings are views and nothing is copied or moved (it was
// rejected as a move out of a borrow).

module Test

struct R: not Copyable {
    var id: Int64;
}

enum E: not Copyable {
    case A(R)
    case B
}

@main
func main() -> lang.i64 {
    let e = E.A(R(id: 7));
    let r = &e;
    let n = match r { .A(x) => x.id, .B => 0 };
    if n != 7 { return 1 }
    0
}
