// test: execution
// stdlib: true
// backends: cranelift,llvm

// Matching on a named ref binding (`let m = &xs(i); match m`) reads the
// referent in place, like `match xs(i)`: the whole element is never
// cloned, and payload binders borrow. It used to copy the referent out
// first, running the clone shim on every match.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Tracked: Cloneable {
    var payload: String
    var clones: Pointer[Int64]
    func clone() -> Tracked {
        unsafe self.clones.write(self.clones.read() + 1);
        Tracked(payload: self.payload.clone(), clones: self.clones)
    }
}

enum Member {
    case Field(key: Tracked, isStatic: Bool)
    case Method(key: Tracked, body: Tracked)
}

func tags(xs: [Member]) -> Int64 {
    var n: Int64 = 0;
    for i in 0..<xs.count {
        let m = &xs(i);
        match m {
            .Field(_, s) => if s { n = n + 100; },
            .Method(_, _) => { n = n + 1; }
        }
    }
    n
}

func payloads(xs: [Member]) -> Int64 {
    var n: Int64 = 0;
    for i in 0..<xs.count {
        let m = &xs(i);
        match m {
            .Field(key, _) => if key.payload == "k" { n = n + 10; },
            .Method(key, body) => if key.payload == body.payload { n = n + 1; }
        }
    }
    n
}

@main
func main() -> lang.i64 {
    let clones = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    unsafe clones.write(0);
    let t = Tracked(payload: "k", clones: clones);
    let xs = [
        Member.Field(key: t.clone(), isStatic: true),
        Member.Method(key: t.clone(), body: t.clone())
    ];
    let c0 = unsafe clones.read();
    let a = tags(xs);
    let c1 = unsafe clones.read();
    let b = payloads(xs);
    let c2 = unsafe clones.read();

    if a != 101 { return 1; }
    if b != 11 { return 2; }
    if c1 != c0 { return 3; }
    if c2 != c1 { return 4; }
    0
}
