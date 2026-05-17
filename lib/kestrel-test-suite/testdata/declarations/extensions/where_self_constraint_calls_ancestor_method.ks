// test: diagnostics
// stdlib: false

// A `where Self: P` extension should be able to call methods declared on
// P's ancestor protocols, not just P itself. Exercises parent-protocol
// lookup through the where-Self constraint path.

module Test

protocol Equatable {
    func isEqual(to other: Self)
}

protocol Comparable: Equatable {
    func compare(other: Self)
}

protocol Container {
    func count()
}

extend Container where Self: Comparable {
    func checkBoth() {
        self.compare(self);  // from Comparable — always worked
        self.isEqual(to: self);   // from Equatable (parent of Comparable) — was broken
    }
}
