// test: execution
// stdlib: true
// expect-stdout: 42\n

// G29 positive control: `Item.Out = Int64` where exactly one of this
// holder's bounds on `Item` (`HasOutA`, not `HasOther`) declares `Out`. The
// clause resolves through the holder-bounds pass, is kept, and pins
// `outA()`'s result to Int64 — no E479, and it runs.

module Test

import std.numeric.Int64

protocol HasOutA { type Out; func outA() -> Out }
protocol HasOther { func other() -> Int64 }
protocol Source { type Item; func fetch() -> Item }

extend Source where Item: HasOutA, Item: HasOther, Item.Out = Int64 {
    public func viaA() -> Int64 { self.fetch().outA() + self.fetch().other() }
}

struct Thing: HasOutA, HasOther {
    type Out = Int64;
    let n: Int64;
    func outA() -> Int64 { self.n }
    func other() -> Int64 { 1 }
}

struct Box: Source {
    type Item = Thing;
    func fetch() -> Thing { Thing(n: 41) }
}

@main
func main() {
    print(Box().viaA());
}
