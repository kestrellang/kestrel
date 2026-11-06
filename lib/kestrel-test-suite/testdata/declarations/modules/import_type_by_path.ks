// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// `import A.B.T` imports member `T` of module `A.B`, like `import A.B.(T)`,
// and `as` renames it.

module Test

import std.numeric.Int64 as Num

@main
func main() -> lang.i64 {
    let x: Num = 41;
    if x + 1 != 42 { return 1 }
    0
}
