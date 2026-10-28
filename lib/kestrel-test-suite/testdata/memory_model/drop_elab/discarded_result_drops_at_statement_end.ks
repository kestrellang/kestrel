// test: execution
// backends: cranelift,llvm
// expect-stdout: drop 1\nafter 1\ndrop 2\nafter 2\n

// A discarded owned result (`mk();`) drops at the end of its statement, not
// at the end of the enclosing scope (drop-semantics.md §6).

module Test

struct R: not Copyable {
    var id: Int64;
    deinit { println("drop \(self.id)"); }
}

func mk(id: Int64) -> R { R(id: id) }

@main
func main() -> lang.i64 {
    mk(1);
    println("after 1");
    mk(2);
    println("after 2");
    0
}
