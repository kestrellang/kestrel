// test: diagnostics
// stdlib: false

// Regression: `extend ConcreteType: Proto[T]` introduces `T` as a free type
// param on the protocol RHS. When a generic function returns `I.Output`
// through a `where I: Proto[T]` bound, the *call-site* projection must
// resolve `Output` through the witness — substituting the extension's free
// `T` with the caller's concrete protocol arg.
//
// Before the fix, `solve_associated` built a substitution from the
// container's TypeParams only (Slot has none). The body's call
// `idx.compute(witness: witness)` returned `I.Output` (an abstract
// projection); at the call site, projecting `I.Output` with `I = Slot`
// dropped the extension's free `T`, mismatching the expected return type.
//
// This is a `diagnostics`-mode test: the goal is that type inference
// emits no errors. Runtime monomorphization of the extension's free type
// param through a witness call is a separate (orthogonal) wiring concern.

module Test

struct Holder[T] {
    var value: T
    public init(value value: T) { self.value = value }
    public func read() -> T { self.value }
}

protocol Indexable[T] {
    type Output
    func compute(witness witness: Holder[T]) -> Output
}

struct Slot { public init() {} }

extend Slot: Indexable[T] {
    type Indexable[T].Output = T
    public func compute(witness witness: Holder[T]) -> T { witness.read() }
}

func project[I, T](idx: I, witness witness: Holder[T]) -> I.Output where I: Indexable[T] {
    idx.compute(witness: witness)
}

func main() -> lang.i64 {
    let s = Slot();
    let h = Holder[lang.i64](value: 42);
    // Type inference must resolve `I.Output` to `lang.i64` via the witness's
    // protocol arg. Without the fix, this errors with "expected T got i64".
    let v: lang.i64 = project(s, witness: h);
    v
}
