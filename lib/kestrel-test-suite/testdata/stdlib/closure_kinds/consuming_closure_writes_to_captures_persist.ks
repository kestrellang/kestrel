// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A `consuming` closure owns its captures for its one call. Writes to a
// captured var (field assignment, mutating method) must be visible later in
// the same call; moving a capture out still drops it exactly once.

module Test

public var drops: Int64 = 0;

struct Counter { var n: Int64; mutating func bump() { self.n = self.n + 1; } }

struct Res: not Copyable {
    var id: Int64;
    deinit { drops = drops + 1; }
}

func take(consuming r: Res) -> Int64 { r.id }

func run(consuming f: consuming () -> Int64) -> Int64 { f() }

@main
func main() -> lang.i64 {
    var c = Counter(n: 0);
    let r1 = run({ c.n = 5; c.bump(); c.n });
    if r1 != 6 { return 1 }
    if c.n != 0 { return 2 }

    var parts = [1];
    let r2 = run({ parts.append(2); parts.count });
    if r2 != 2 { return 3 }
    if parts.count != 1 { return 4 }

    let res = Res(id: 9);
    let r3 = run({ take(res) });
    if r3 != 9 { return 5 }
    if drops != 1 { return 6 }
    0
}
