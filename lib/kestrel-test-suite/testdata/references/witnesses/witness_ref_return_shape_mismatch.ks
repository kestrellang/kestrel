// test: diagnostics
// stdlib: true

// Stage 2d: reference returns must match a protocol requirement EXACTLY in
// shape and mutability. `-> &T` is a borrow return (raw pointer ABI) — an
// owned `-> T` impl cannot witness it, an owned requirement cannot be
// witnessed by a `-> &T` impl, and `&` never satisfies `&mutating`. Each
// near-miss is E458, not a silent ABI mismatch.
module Test

protocol RefGetter {
    type Item
    func fetch() -> &Item
}

protocol OwnedGetter {
    type Item
    func grab() -> Item
}

protocol MutGetter {
    type Item
    mutating func access() -> &mutating Item
}

// Impl returns owned where the requirement is a shared ref.
struct A: RefGetter {
    type Item = Int64
    var v: Int64

    func fetch() -> Int64 { // ERROR(E458)
        self.v
    }
}

// Impl returns a shared ref where the requirement is owned.
struct B: OwnedGetter {
    type Item = Int64
    var v: Int64

    func grab() -> &Int64 { // ERROR(E458)
        self.v
    }
}

// Impl returns a shared ref where the requirement is a mutating ref.
struct C: MutGetter {
    type Item = Int64
    var v: Int64

    mutating func access() -> &Int64 { // ERROR(E458)
        self.v
    }
}
