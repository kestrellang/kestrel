// test: execution
// stdlib: true
// expect-stdout: 111 151 1 5 1 | 114 3 6\n

// A protocol extension gives default `ref` / `mutating ref` bodies: for a
// requirement (used unless the conformer declares its own, through generic
// code and directly) and for an extension-only place.
module Test

protocol Store {
    var raw: Int64 { ref mutating ref }
    var doubled: Int64 { ref }
    subscript(at index: Int64) -> Int64 { ref mutating ref }
}

extend Store {
    // Defaults for the requirements.
    var doubled: Int64 { ref { self.raw } }
    subscript(at index: Int64) -> Int64 {
        ref { self.raw }
        mutating ref { self.raw }
    }
    // Extension-only place (not a requirement).
    var alias: Int64 {
        ref { self.raw }
        mutating ref { self.raw }
    }
}

struct A: Store { var raw: Int64 }

struct B: Store {
    var raw: Int64
    var other: Int64
    subscript(at index: Int64) -> Int64 {
        ref { self.other }
        mutating ref { self.other }
    }
}

func read[S](s: &S) -> Int64 where S: Store { s.doubled * 100 + s(at: 0) * 10 + s.alias }
func bump[S](mutating s: S) where S: Store { s(at: 0) += 1; s.alias += 2; }

@main
func main() -> lang.i64 {
    var a = A(raw: 1);
    var b = B(raw: 1, other: 5);
    print("\(read(a)) \(read(b)) \(a(at: 0)) \(b(at: 0)) \(a.alias)");
    bump(a);
    bump(b);
    a(at: 0) += 10;
    a.alias += 100;
    print(" | \(a.raw) \(b.raw) \(b.other)\n");
    0
}
