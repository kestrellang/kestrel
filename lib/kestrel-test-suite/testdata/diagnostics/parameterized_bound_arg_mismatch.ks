// test: diagnostics
// stdlib: false

// Parameterized protocol bounds must enforce their type arguments.
// `where R: Reader[Int16-like]` used to silently accept `Box[i64]` (the
// bound's args were recorded but never unified with the declared
// conformance's args), which mono then miscompiled as a wrong-layout
// read. Fixed in solve_conforms/unify_bound_protocol_args — a definite
// arg conflict is now a clean conformance error, while blanket free
// params (`extend T64: Blank[U]` below) still unify freely.

module Test

protocol Reader[B] {
    func value() -> B
}

struct Box[T] {
    var item: T;
}

extend Box[T]: Reader[T] {
    public func value() -> T { self.item }
}

func readThrough32[R](source: R) -> lang.i32 where R: Reader[lang.i32] {
    source.value()
}

// Blanket free param on the protocol side must keep unifying freely.
protocol Blank[U] {
    func ignore()
}

struct T64 {
    var x: lang.i64;
}

extend T64: Blank[U] {
    public func ignore() {}
}

func wantsBlank[R](source: R) where R: Blank[lang.i64] {
    source.ignore()
}

@main
func main() -> lang.i64 {
    let ok = Box[lang.i32](item: lang.cast_i64_i32(7));
    let a = readThrough32(ok);

    let bad = Box[lang.i64](item: 7);
    let b = readThrough32(bad); // ERROR: does not conform

    let t = T64(x: 1);
    wantsBlank(t);
    0
}
