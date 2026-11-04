// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: field\n  mutating ref\ncompound\n  mutating ref\narg\n  mutating ref\nread\n  ref\nsub field\n  mutating ref\nsub compound\n  mutating ref\nsub arg\n  mutating ref\na.x 8 array 6\n

// A write INTO a member that has a `mutating ref` accessor (`w.value.x = 5`,
// `w.value.x += 1`, passing `w.value.x` to a `mutating` parameter) lends the
// place through `mutating ref`. It used to go through the read-only `ref`
// accessor and write through its `&T`, skipping whatever the write accessor
// does first (a copy-on-write fork). Subscripts too, and the analyzer no
// longer rejects `xs(at: 0).x = 5` as a write through `&T` (E201).
module Test

struct P: Cloneable {
    var x: Int64
    func clone() -> P { P(x: self.x) }
}

func bump(mutating n: Int64) { n += 1 }

struct W {
    var p: Pointer[P]

    var value: P {
        ref { println("  ref"); self.p.value }
        mutating ref { println("  mutating ref"); self.p.mutatingValue }
    }

    subscript(at i: Int64) -> P {
        ref { println("  ref"); self.p.value }
        mutating ref { println("  mutating ref"); self.p.mutatingValue }
    }
}

@main
func main() -> lang.i64 {
    var a = P(x: 1);
    var w = W(p: Pointer(to: a));
    println("field"); w.value.x = 5;
    println("compound"); w.value.x += 1;
    println("arg"); bump(w.value.x);
    println("read"); let _ = w.value.x;
    println("sub field"); w(at: 0).x = 6;
    println("sub compound"); w(at: 0).x += 1;
    println("sub arg"); bump(w(at: 0).x);
    var xs: Array[P] = [P(x: 1)];
    xs(at: 0).x = 5;
    xs(at: 0).x += 1;
    println("a.x \(a.x) array \(xs(at: 0).x)");
    0
}
