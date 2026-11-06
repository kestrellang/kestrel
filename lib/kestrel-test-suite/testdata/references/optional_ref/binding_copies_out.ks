// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: unannotated\n  clone\nwrite after bind\nannotated\n  clone\nref kept\nvar\n  clone\nrefs iterator\n  clone\nf 1 a 1 k 9 v 1 n 1\n

// Binding an `Optional[&T]` copies the payload out, the way binding a `&T`
// does: `let f = b.first` gives `f: Optional[S]`, owned, and the place is
// free again, so writing `b` afterwards doesn't show through `f`. Spelling
// the binding `Optional[&T]` keeps the reference. Each decay clones once.
module Test

struct S: Cloneable {
    var n: Int64
    func clone() -> S {
        println("  clone");
        S(n: self.n)
    }
}

struct B {
    var xs: Array[S]

    var first?: S {
        ref {
            if self.xs.isEmpty { return null; }
            let r = &self.xs(0);
            .Some(r)
        }
    }
}

func n(of o: S?) -> Int64 {
    match o {
        .Some(s) => s.n,
        .None => -1
    }
}

@main
func main() -> lang.i64 {
    var b = B(xs: [S(n: 1)]);
    println("unannotated");
    let f = b.first;
    println("write after bind");
    b.xs(0).n = 9;
    b.xs(0).n = 1;
    println("annotated");
    let a: S? = b.first;
    println("ref kept");
    let k: Optional[&S] = b.first;
    let kn = if let .Some(s) = k { s.n * 9 } else { -1 };
    println("var");
    var v = b.first;
    println("refs iterator");
    var it = b.xs.refs();
    let next = it.next();
    println("f \(n(of: f)) a \(n(of: a)) k \(kn) v \(n(of: v)) n \(n(of: next))");
    v = .None;
    0
}
