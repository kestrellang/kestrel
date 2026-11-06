// test: execution
// backends: cranelift,llvm
// expect-stdout: drop 4\n

// A body whose only statement is `deinit r;` has no expressions; the move
// checker used to panic looking for one to anchor the move to.

module Test

struct R: not Copyable {
    var id: Int64;
    deinit { println("drop \(self.id)"); }
}

func finish(consuming r: R) { deinit r; }

@main
func main() -> lang.i64 {
    finish(R(id: 4));
    0
}
