// test: execution
// stdlib: true
// expect-stdout: 10 20 30 | 11 21 31\n

// A protocol can require several subscript places that differ only by
// label. Each requirement binds the conformer's subscript with the same
// labels, not just the first subscript that has a `ref` accessor.
module Test

protocol Triple {
    subscript(index: Int64) -> Int64 { ref mutating ref }
    subscript(second index: Int64) -> Int64 { ref mutating ref }
    subscript(third index: Int64) -> Int64 { ref mutating ref }
}

struct T3: Triple {
    var a: Int64
    var b: Int64
    var c: Int64
    subscript(index: Int64) -> Int64 {
        ref { self.a }
        mutating ref { self.a }
    }
    subscript(second index: Int64) -> Int64 {
        ref { self.b }
        mutating ref { self.b }
    }
    subscript(third index: Int64) -> Int64 {
        ref { self.c }
        mutating ref { self.c }
    }
}

func show[P](p: &P) where P: Triple {
    print("\(p(0)) \(p(second: 0)) \(p(third: 0))");
}

func bump[P](mutating p: P) where P: Triple {
    p(0) += 1;
    p(second: 0) += 1;
    p(third: 0) += 1;
}

@main
func main() -> lang.i64 {
    var t = T3(a: 10, b: 20, c: 30);
    show(t);
    bump(t);
    print(" | ");
    show(t);
    print("\n");
    0
}
