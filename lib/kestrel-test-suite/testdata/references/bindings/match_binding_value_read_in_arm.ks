// test: execution
// stdlib: true
// backends: cranelift,llvm

// A named ref binding read by value inside an arm of a match on that
// same binding: `[m]`, a struct field, a tuple element, an optional
// payload. The arm sees the binding under a fresh name (threaded into
// the arm's block); each value read must still copy the referent. The
// array literal wrote the borrow out uncopied, an OSSA verify ICE
// (sojourn's compileClass: `try self.compileFieldInitializer([m], ...)`).
module Test

import std.numeric.(Int64)

enum Member {
    case Field(name: String)
    case Block([String])
}

struct W { var m: Member }

struct Err: Error { var code: Int64 }

func count(ms: [Member]) -> Int64 throws Err {
    if ms.count == 0 { throw Err(code: 1); }
    ms.count
}
func one(w: W) -> Int64 { 1 }
func pair(p: (Member, Int64)) -> Int64 { p.1 }
func opt(o: Member?) -> Int64 { if let some _ = o { 1 } else { 0 } }

func each(members: [Member]) -> Int64 throws Err {
    var total: Int64 = 0;
    for i in 0..<members.count {
        let m = &members(i);
        match m {
            .Field(_) => {
                total = total + try count([m]);
                total = total + one(W(m: m));
                total = total + pair((m, 1));
                total = total + opt(m);
                let c = m;
                total = total + try count([c, m]);
            },
            _ => {}
        }
    }
    total
}

func pick(members: [Member]) -> Member {
    let m = &members(0);
    match m {
        .Field(_) => m,
        _ => Member.Block([])
    }
}

@main
func main() -> lang.i64 {
    let ms = [Member.Field(name: "a"), Member.Block(["x"]), Member.Field(name: "b")];
    let n = match each(ms) {
        .Ok(n) => n,
        .Err(_) => -1
    };
    if n != 12 { return 1; }
    if let .Field(name) = pick(ms) {
        if name != "a" { return 2; }
    } else {
        return 3;
    }
    0
}
