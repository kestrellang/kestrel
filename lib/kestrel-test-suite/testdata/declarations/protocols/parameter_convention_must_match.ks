// test: diagnostics
// stdlib: false

// A witness's parameter conventions must match the requirement's exactly,
// like its receiver (E477). A `consuming` witness behind a borrowing
// requirement would destroy a value its caller still owns; a borrowing
// witness of a `consuming` requirement silently changes who drops it.

module Test

struct Tok: not Copyable {
    var id: lang.i64
}

protocol Sink { func put(consuming t: Tok) }
protocol Peek { func look(t: Tok) -> lang.i64 }
protocol Bump { func bump(mutating n: lang.i64) }

struct A: Sink {
    func put(t: Tok) { } // ERROR(E469)
}

struct B: Peek {
    func look(consuming t: Tok) -> lang.i64 { t.id } // ERROR(E469)
}

struct C: Bump {
    func bump(n: lang.i64) { } // ERROR(E469)
}

// Matching conventions are accepted.
struct D: Sink, Peek, Bump {
    func put(consuming t: Tok) { }
    func look(t: Tok) -> lang.i64 { t.id }
    func bump(mutating n: lang.i64) { n = 0; }
}
