// test: execution
// stdlib: true
// expect-exit: 0
//
// Parent tables and composition bounds are ordered by qualified name, not by
// declaration (entity) order. `Zebra` is declared before `Apple` but sorts
// after it: every call, upcast and composition must still reach the right
// table.

module Test

import std.numeric.(Int64)

protocol Zebra {
    func stripes() -> Int64
}

protocol Apple {
    func seeds() -> Int64
}

protocol Both: Zebra, Apple {
    func total() -> Int64
}

struct Thing { var a: Int64; var z: Int64; }
extend Thing: Both {
    public func stripes() -> Int64 { self.z }
    public func seeds() -> Int64 { self.a }
    public func total() -> Int64 { self.a + self.z }
}

func viaZebra(x: any Zebra) -> Int64 { x.stripes() }
func viaApple(x: any Apple) -> Int64 { x.seeds() }

@main
func main() -> Int64 {
    let b: any Both = Thing(a: 3, z: 40);
    if b.stripes() != 40 { return 1 }
    if b.seeds() != 3 { return 2 }
    if b.total() != 43 { return 3 }
    if viaZebra(b) != 40 { return 4 }
    if viaApple(b) != 3 { return 5 }
    let c: any Zebra and Apple = Thing(a: 5, z: 70);
    if c.stripes() != 70 { return 6 }
    if c.seeds() != 5 { return 7 }
    if viaZebra(c) != 70 { return 8 }
    if viaApple(c) != 5 { return 9 }
    0
}
