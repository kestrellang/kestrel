// test: execution
// stdlib: true
// backends: cranelift,llvm

// Clone-count pin for place-mode matches: `&` bindings contribute ZERO
// clones (in-place projection), plain bindings in the same arm force
// exactly one copy each. The guard reads pattern variables through raw
// projection views — also clone-free.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Tracked: Cloneable {
    var payload: String
    var clones: Pointer[Int64]
    func clone() -> Tracked {
        self.clones.write(self.clones.read() + 1);
        Tracked(payload: self.payload.clone(), clones: self.clones)
    }
}

enum Box2 {
    case Pair(Tracked, Tracked)
    case Nothing
}

@main
func main() -> lang.i64 {
    let clones = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    clones.write(0);
    let bx = Box2.Pair(
        Tracked(payload: "a", clones: clones),
        Tracked(payload: "b", clones: clones)
    );

    let c0 = clones.read();
    let got = match bx {
        .Pair(&a, b) if a.payload == "a" => b.payload,
        .Pair(&a, b) => a.payload,
        .Nothing => "x"
    };
    let c1 = clones.read();

    if got != "b" { return 1; }
    if c1 - c0 != 1 { return 2; }
    0
}
