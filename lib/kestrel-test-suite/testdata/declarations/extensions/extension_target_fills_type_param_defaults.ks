// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 3 7 1\n

// An extension that names fewer type args than the target declares gets the
// declared defaults for the rest (Rust's rule): `extend Pair[T]` extends
// `Pair[T, Int64]` only, and `extend Array[T]` extends `Array[T, GlobalAllocator]`.
module Test

struct Pair[T, U = Int64] {
    var first: T
    var second: U
}

extend Pair[T] {
    func secondPlusOne() -> Int64 { self.second + 1 }
}

extend Array[T] {
    func countTimesTwo() -> Int64 { self.count * 2 }
}

extend [T] {
    func countMinusOne() -> Int64 { self.count - 1 }
}

@main
func main() {
    let p = Pair[Bool](first: true, second: 2);
    let xs: [Int64] = [1, 2];
    let ys = [1, 2, 3, 4];
    print("\(p.secondPlusOne()) \(xs.countTimesTwo() + ys.countMinusOne()) \(xs.countMinusOne())\n");
}
