// test: execution
// stdlib: true
// expect-stdout: 5 7\n0 0\n

module Test

import std.ffi.(malloc, free)
import std.memory.(RawPointer)

// A `clone()` that returns a mutated `var` from an `if` branch used to free
// the clone's buffer twice. The branch tail copied `o` instead of moving it,
// and inside a type's own `clone()` that copy is a plain bit copy, so the
// returned value and the scope-exit drop of `o` shared one pointer. Branch
// tails in return position now move the local.
struct Owner: Cloneable {
    var p: RawPointer
    var n: Int64
    init(p p: RawPointer) { self.p = p; self.n = 0; }
    deinit { unsafe free(self.p) }
    func clone() -> Owner {
        var outer = Owner(p: unsafe malloc(8));
        outer.n = self.n;
        if self.n > 6 {
            outer
        } else if self.n > 0 {
            var inner = Owner(p: unsafe malloc(8));
            inner.n = self.n;
            inner
        } else {
            match self.n { 0 => Owner(p: unsafe malloc(8)), _ => outer }
        }
    }
}

@main
func main() {
    var a = Owner(p: unsafe malloc(8));
    a.n = 5;
    var b = Owner(p: unsafe malloc(8));
    b.n = 7;
    let c = Owner(p: unsafe malloc(8));
    print("\(a.clone().n) \(b.clone().n)\n");
    print("\(c.clone().n) \(c.n)\n");
}
