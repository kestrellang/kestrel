// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: rng 2 20 view 3 30 rows 12 write 99\n

// Binding an unsized place keeps the reference: `let rng = xs(0..<2)` is a
// `&Slice[T]` (there is no value to copy out), the same as writing
// `let rng = &xs(0..<2)`. Sized places still copy out (`let row = m(i)`).
module Test
import std.collections.(Array)

@main
func main() -> Int64 {
    var arr: Array[Int64] = [10, 20, 30];
    let rng = arr(0..<2);
    let view = arr.asSlice();
    print("rng \(rng.count) \(rng(1)) view \(view.count) \(view(2)) ");

    // An index whose type comes from a literal resolves late; the binding
    // and the nested subscript still decay to plain values.
    let sigma: [Array[Int64]] = [[2, 0], [1, 1]];
    let m: Array[Int64] = [5, 7];
    var t: Int64 = 0;
    for round in 0..<2 {
        let row = sigma(round % 2);
        t = t + m(row(1));
    }
    print("rows \(t) ");

    arr(1..)(0) = 99;
    print("write \(arr(1))\n");
    0
}
