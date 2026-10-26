// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A type nested 100 levels deep compiles. Every level is its own
// instantiation, and each symbol name spelled out the whole type, so symbol
// bytes grew with the square of the depth (800 levels took 3 GB). Long
// symbols are now shortened to a prefix plus a hash.

module Test

@main
func main() -> lang.i64 {
    let x: Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Int64]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]] = [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[7]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]];
    if x.count != 1 { return 1 }
    0
}
