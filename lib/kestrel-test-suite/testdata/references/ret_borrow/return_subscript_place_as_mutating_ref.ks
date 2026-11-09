// test: execution
// stdlib: true
// expect-stdout: 5 6 11\n[10, 2] [1, 2]\n[10, 20] [10, 2]\n[8, 2] [1, 2]\n

// A subscript or computed-property read returned as `&mutating T` lends the
// member's `mutating ref` place. It used to be typed shared `&T` and fail
// with E109; the read must also call the `mutating ref` accessor, so Array's
// copy-on-write separates the buffer from earlier copies.
module Test

struct V {
    var x: Int64
    subscript(i: Int64) -> Int64 { ref { self.x } mutating ref { self.x } }
}

struct C {
    var v: V
    var items: [Int64]
    mutating func slot() -> &mutating Int64 { self.v(0) }
    var first: Int64 { ref { self.items(0) } mutating ref { self.items(0) } }
}

func f(mutating xs: [Int64]) -> &mutating Int64 { xs(0) }
func g(mutating xs: [Int64]) -> &mutating Int64 { return xs(0); }

struct P {
    var xs: [Int64]
    var first: Bool
    mutating func pick() -> &mutating Int64 {
        if self.first { return self.xs(0); }
        self.xs(1)
    }
}

struct W {
    var xs: [Int64]
    var head: Int64 { ref { self.xs(0) } mutating ref { self.xs(0) } }
}

struct O {
    var w: W
    var h: Int64 { ref { self.w.head } mutating ref { self.w.head } }
    mutating func hp() -> &mutating Int64 { self.w.head }
}

@main
func main() -> Int64 {
    var c = C(v: V(x: 1), items: [1, 2]);
    c.slot() = 5;
    c.first = 6;
    var zs = [1, 2];
    f(zs) = 10;
    g(zs) += 1;
    println("\(c.v.x) \(c.items(0)) \(zs(0))");

    var ys = [1, 2];
    let copy = ys;
    f(ys) = 10;
    println("\(ys) \(copy)");

    var p = P(xs: ys, first: false);
    let pc = p.xs;
    p.pick() = 20;
    println("\(p.xs) \(pc)");

    var o = O(w: W(xs: [1, 2]));
    let oc = o.w.xs;
    o.h = 7;
    o.hp() += 1;
    println("\(o.w.xs) \(oc)");
    0
}
