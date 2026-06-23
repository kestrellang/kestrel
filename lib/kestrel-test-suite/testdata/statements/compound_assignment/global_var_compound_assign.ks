// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#140): `g += 1` on a module-level global desugars to a
// `mutating addAssign` call whose receiver place was a loaded snapshot of the
// global, so the mutation hit a throwaway copy and the global never changed.
// `lower_place` now resolves a stored-global `Def` to its GlobalRef address,
// so the compound-assign receiver borrows the real global. The global is
// declared AFTER the function that mutates it to pin the forward-reference
// case (the global isn't in `module.statics` yet while `bump` lowers, so the
// resolution must be by AST shape, not the statics map).

module Test

import std.num.Int64

@main
func main() -> Int64 {
    bump();
    bump();
    if counter != 2 { return 1 };
    counter += 40;
    if counter != 42 { return 2 };
    counter = counter + 8;   // plain assign still works
    if counter != 50 { return 3 };
    0
}

func bump() {
    counter += 1;
}

var counter: Int64 = 0;
