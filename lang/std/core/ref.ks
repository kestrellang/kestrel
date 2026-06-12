// Reference conformances
// References forward comparison protocols to their pointees: `&T` compares
// as the VALUE it points at, never by address.

module std.core

import std.core.(Equatable, Comparable, Ordering, Bool)

/// A shared reference compares as its pointee: `&a == &b` (and ref-payload
/// containers like `Optional[&T]`) dispatch the POINTEE's `Equatable`.
/// The receiver peels transparently; `other` rides the borrow-convention
/// pass-through — no copy of the pointee is made.
extend &T: Equatable where T: Equatable {
    public func isEqual(to other: Self) -> Bool {
        self.isEqual(to: other)
    }
}

/// A mutating reference compares the same way (read access is implied).
/// Each mutability conforms via its own extension — there is no
/// `&mutating` → `&` subsumption.
extend &mutating T: Equatable where T: Equatable {
    public func isEqual(to other: Self) -> Bool {
        self.isEqual(to: other)
    }
}

/// A shared reference orders as its pointee. `Comparable` refines
/// `Equatable`; the `Equatable` half comes from the extension above
/// (`T: Comparable` implies `T: Equatable`).
extend &T: Comparable where T: Comparable {
    public func compare(other: Self) -> Ordering {
        self.compare(other)
    }
}

/// A mutating reference orders the same way.
extend &mutating T: Comparable where T: Comparable {
    public func compare(other: Self) -> Ordering {
        self.compare(other)
    }
}
