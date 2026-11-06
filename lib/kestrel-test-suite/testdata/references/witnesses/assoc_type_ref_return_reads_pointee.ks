// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: fn 2 method 2 subscript 2 bound 2 write 7 10 10\n

// A generic return that is a ref only once instantiated (`-> I.Ref` with
// `Int64.Ref = &V`) hands back a pointer. The caller must read through it
// like a declared `-> &V` call; it used to read the pointer bits as the
// value. The `&mutating` form writes through to the original.
module Test

import std.collections.(Array)

struct V { var n: Int64 }

protocol Slot {
    type Ref
    type MutRef
    func read(w: W) -> Ref
    func write(mutating w: W) -> MutRef
}

extend Int64: Slot {
    type Ref = &V
    type MutRef = &mutating V
    func read(w: W) -> &V { w.items(self) }
    func write(mutating w: W) -> &mutating V {
        let r = &mutating w.items(self);
        r
    }
}

struct W {
    var items: Array[V]
    subscript[I](index: I) -> I.Ref where I: Slot {
        get { index.read(self) }
    }
    func viaMethod[I](index: I) -> I.Ref where I: Slot { index.read(self) }
}

func fetch[I](i: I, w: W) -> I.Ref where I: Slot { i.read(w) }
func put[I](i: I, mutating w: W) -> I.MutRef where I: Slot { i.write(w) }

@main
func main() -> Int64 {
    var w = W(items: [V(n: 1), V(n: 2)]);
    let i: Int64 = 1;
    let bound = fetch(i, w);
    print("fn \(fetch(i, w).n) method \(w.viaMethod(i).n) subscript \(w(i).n) bound \(bound.n)");
    let before = fetch(0, w);
    put(0, w).n = 9;
    put(0, w).n += 1;
    print(" write \(before.n + 6) \(fetch(0, w).n) \(w.items(0).n)\n");
    0
}
