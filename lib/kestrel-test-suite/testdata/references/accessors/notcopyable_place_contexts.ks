// test: execution
// stdlib: true
// backends: cranelift,llvm

// Strongest misclassification pin: a NotCopyable element compiles for
// every PLACE-context use of a ref accessor — member reads, mutating
// methods, borrow arguments. A place context misclassified as a value
// context fails HERE at compile time (the copy guard), not as a silent
// clone.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Res: not Copyable {
    var v: Int64
    func peekV() -> Int64 { self.v }
    mutating func bump() { self.v = self.v + 1; }
}

struct Buf {
    var p: Pointer[Res]
    subscript(at index: Int64) -> Res {
        ref { self.p.offset(by: index).value }
        mutating ref { self.p.offset(by: index).mutatingValue }
    }
}

func inspect(r: Res) -> Int64 { r.peekV() }

@main
func main() -> lang.i64 {
    let p = SystemAllocator().allocate(Layout.of[Res]()).unwrap().cast[Res]();
    p.write(Res(v: 5));
    var b = Buf(p: p);

    if b(at: 0).peekV() != 5 { return 1; }
    if inspect(b(at: 0)) != 5 { return 2; }
    b(at: 0).bump();
    if b(at: 0).peekV() != 6 { return 3; }
    b(at: 0) = Res(v: 9);
    if b(at: 0).peekV() != 9 { return 4; }
    0
}
