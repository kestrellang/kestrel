// test: execution
// backends: cranelift,llvm
// stdlib: true
// expect-stdout: 2 8\n3\n

// A call returning a ref-carrying value borrows only the arguments whose
// type can hold what the result points at. An `Int64` can't hold an
// `Array`, so literal and local `Int64` arguments don't scope the result:
// returning `mk(xs, 1)` is fine (no E494 on the `1` temporary), and storing
// `mk(xs, i)` into an outer variable is fine (no E509 on the inner `i`).
// A method may still return a borrow of a non-receiver parameter.
module Test

struct View { let base: &Array[Int64]; let lo: Int64 }

extend Array[Int64] {
    func other(o: [Int64]) -> View { let b = &o; View(base: b, lo: 0) }
}

func mk(xs: [Int64], lo: Int64) -> View { let b = &xs; View(base: b, lo: lo) }
func viaFree(xs: [Int64]) -> View { mk(xs, 1) }
func viaOther(xs: [Int64], ys: [Int64]) -> View { xs.other(ys) }

@main
func main() {
    let xs: [Int64] = [1, 2, 3];
    let ys: [Int64] = [7, 8];
    let v = viaFree(xs);
    println("\(v.base(v.lo)) \(viaOther(xs, ys).base(1))");
    var outer = mk(xs, 0);
    if true {
        let i: Int64 = 2;
        outer = mk(xs, i);
    }
    println("\(outer.base(outer.lo))");
}
