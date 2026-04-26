// test: diagnostics
// stdlib: false

// Gating probe for the Array `ArrayIndex[T]` refactor:
// One generic subscript with a where-clause-bound index type. Two distinct
// types conform to the protocol; both must dispatch through the same
// generic subscript at the call site, and `I.Output` must resolve correctly
// per conformance.

module Test

protocol IndexLike[T] {
    type Output
    func resolveIn(holder holder: Holder[T]) -> Output
}

struct Holder[T] {
    var value: T
    public init(value value: T) { self.value = value }
    public func read() -> T { self.value }
}

struct IntKey { public init() {} }
struct WrapKey { public init() {} }

extend IntKey: IndexLike[T] {
    type IndexLike[T].Output = T
    public func resolveIn(holder holder: Holder[T]) -> T { holder.read() }
}

struct Pair[T] {
    var a: T
    var b: T
    public init(a a: T, b b: T) { self.a = a; self.b = b }
}

extend WrapKey: IndexLike[T] {
    type IndexLike[T].Output = Pair[T]
    public func resolveIn(holder holder: Holder[T]) -> Pair[T] {
        Pair[T](a: holder.read(), b: holder.read())
    }
}

struct Bag[T] {
    var inner: Holder[T]
    public init(value value: T) { self.inner = Holder[T](value: value) }

    // Single generic subscript — handles every conforming index type.
    public subscript[I](pick pick: I) -> I.Output where I: IndexLike[T] {
        get { pick.resolveIn(holder: self.inner) }
    }
}

func main() -> lang.i64 {
    let bag = Bag[lang.i64](value: 7);

    // IntKey → Output = T → lang.i64
    let one: lang.i64 = bag(pick: IntKey());
    if lang.i64_eq(one, 7) {} else { return 1 }

    // WrapKey → Output = Pair[T] → Pair[lang.i64]
    let two: Pair[lang.i64] = bag(pick: WrapKey());
    if lang.i64_eq(two.a, 7) {} else { return 2 }
    if lang.i64_eq(two.b, 7) {} else { return 3 }

    0
}
