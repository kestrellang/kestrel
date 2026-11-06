// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: 13 3\n

// An implicit slice binding (`let mid = ys(1..<4)`, a `&Slice[T]`) whose
// first use is as an argument to a `Slice[T]` param. Not working yet: the
// range's element type comes from a literal, so the binding's pointee is
// still unresolved when the argument pins the binding to `Slice[Int64]`,
// and lowering then copies the slice out (E503). `let mid = &ys(1..<4)`
// works today.
module Test
import std.collections.(Array, Slice)

func sum(s: Slice[Int64]) -> Int64 {
    var t: Int64 = 0;
    for x in s { t += x; }
    t
}

@main
func main() -> Int64 {
    var ys: Array[Int64] = [5, 3, 9, 1, 7];
    let mid = ys(1..<4);
    print("\(sum(mid)) \(mid.count)\n");
    0
}
