// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: sum 9 3 utf8 hi count 4 5 append 4 starts true\n

// A generic `Iterable` param that relaxes `Sized` and `Copyable` takes a
// range place directly: `S` binds to the unsized `Slice[T]` and the slice
// is passed by address (a borrowed param), so nothing is copied. A
// `&Slice[T]` param takes the place too, and an `Array` peels to one.
module Test
import std.collections.(Array, Slice)
import std.iter.(Iterable)

func total[S](s: &S) -> Int64 where S: Iterable, S.Item = Int64, S: not Static, S: not Copyable, S: not Sized {
    var t: Int64 = 0;
    for x in s { t = t + x }
    t
}

func count(s: &Slice[Int64]) -> Int64 {
    s.count
}

@main
func main() -> Int64 {
    var xs: Array[Int64] = [1, 2, 3, 4, 5];
    let bytes: Array[UInt8] = [104, 105, 33];
    let str = String(fromUtf8: bytes(0..<2)) ?? String();
    var ys: Array[Int64] = [9, 9];
    ys.append(contentsOf: xs(3..));
    print("sum \(total(xs(1..<4))) \(total(xs(..<2))) utf8 \(str) ");
    print("count \(count(xs(1..))) \(count(xs)) append \(ys.count) ");
    print("starts \(xs(..<3).starts(with: xs(0..<2)))\n");
    0
}
