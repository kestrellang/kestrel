// test: execution
// backends: cranelift,llvm
// stdlib: true
// expect-stdout: 5 1\nxs 99 12 ys 1 2\n

// Subscripts called through a reference FIELD (`self.base(i)` where
// `base: &Array[T]`) dispatch on the pointee, not on `&Array`. Reads work
// through a `&` field; writes and `+=` work through a `&mutating` field and
// land in the referenced array (not a copy), still running its
// copy-on-write check (`ys` keeps its old contents). Writing needs a
// mutable holder: a `&mutating` field gives no interior mutability.
module Test

struct View { let base: &Array[Int64] }

struct MutView {
    let base: &mutating Array[Int64]
    mutating func put(i: Int64, v: Int64) { self.base(i) = v; }
    mutating func bump(i: Int64) { self.base(i) += 10; }
}

func viaField(s: View) -> Int64 { s.base(1) + s.base(2) }

@main
func main() {
    var xs: [Int64] = [1, 2, 3];
    let b = &xs;
    let s = View(base: b);
    println("\(viaField(s)) \(s.base(0))");
    let mb = &mutating xs;
    var m = MutView(base: mb);
    let ys = xs;
    m.put(0, 99);
    m.bump(1);
    println("xs \(xs(0)) \(xs(1)) ys \(ys(0)) \(ys(1))");
}
