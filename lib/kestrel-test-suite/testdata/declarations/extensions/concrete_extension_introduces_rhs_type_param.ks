// test: execution
// stdlib: false

// `extend ConcreteType: GenericProto[T]` — `T` has no source on the LHS
// (Slot is non-generic), so the extension itself introduces `T` as a free
// type parameter. The conformance is "for all T, Slot conforms to
// Indexable[T]". Method bodies use `T`, and at the call site `T` is
// inferred from arguments.

module Test

protocol Indexable[T] {
    type Output
    func fetch(holder holder: Holder[T]) -> Output
}

struct Holder[T] {
    var value: T
    public init(value value: T) { self.value = value }
    public func read() -> T { self.value }
}

struct Slot { public init() {} }

extend Slot: Indexable[T] {
    type Indexable[T].Output = T
    public func fetch(holder holder: Holder[T]) -> T { holder.read() }
}

@main
func main() -> lang.i64 {
    let h = Holder[lang.i64](value: 42);
    let s = Slot();
    let v = s.fetch(holder: h);
    if lang.i64_eq(v, 42) { 0 } else { 1 }
}
