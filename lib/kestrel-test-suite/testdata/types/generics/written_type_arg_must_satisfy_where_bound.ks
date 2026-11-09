// test: diagnostics
// stdlib: false

// A generic type that is only written or built (no call checks its where
// clauses) still has to satisfy the bounds on its type parameters. The
// error lands where the type is written, not inside the type's own body
// after monomorphization.

module Test

protocol Keyed {}

struct Good: Keyed {}
struct Bad {}

struct Table[K] where K: Keyed {
    init() {}
}

struct Holder[T] {
    init() {}
}

func annotated() {
    let t: Table[Bad] = Table(); // ERROR: Bad !: Keyed
}

func constructed() {
    let t = Table[Bad](); // ERROR: Bad !: Keyed
}

func returned() -> Table[Bad] { // ERROR: Bad !: Keyed
    Table()
}

func nested() {
    let t: Holder[Table[Bad]] = Holder(); // ERROR: Bad !: Keyed
}

func fine() {
    let t: Table[Good] = Table();
    let u = Table[Good]();
}
