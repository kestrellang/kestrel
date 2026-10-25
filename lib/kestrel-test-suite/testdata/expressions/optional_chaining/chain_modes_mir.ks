// test: mir
// stdlib: true
// mir-filter: Test.modes

// Each `?.` mode calls its own method: a read on a place borrows
// (`chainBorrow`), a mutation of a place lends mutably (`chainMutate`), and
// a temporary hands its payload over (`chainTake`).

module Test

struct Counter {
    var n: Int64
    mutating func bump() { self.n = self.n + 1; }
}

func make() -> Counter? { .Some(Counter(n: 1)) }

func modes() {
    var c: Counter? = .Some(Counter(n: 1));
    let read = c?.n;
    c?.bump();
    let taken = make()?.n;
}
