// test: execution
// stdlib: true
// expect-exit: 0

// #184: a generic function with an associated-type-projection bound
// (`where T: Producer, T.Item: Show`) calls a witness on the projected value
// (`x.produce().show()`). The projection `T.Item` must keep its base `T` so
// that at monomorphization (T = IntSrc) it reduces to the concrete `Item`
// (Int64) and `show` dispatches concretely. Collapsing the bound subject to
// the bare `Item` entity made `produce()` resolve to a baseless `Param(Item)`
// that leaked past mono (TypeParam ICE + unresolved witness).

module Test

import std.text.String
import std.numeric.Int64

protocol Show { func show() -> String }
extend Int64: Show { public func show() -> String { "i" } }

protocol Producer {
    type Item
    func produce() -> Item
}

struct IntSrc { var v: Int64; }
extend IntSrc: Producer {
    public type Item = Int64;
    public func produce() -> Int64 { self.v }
}

func render[T](x: T) -> String where T: Producer, T.Item: Show {
    x.produce().show()
}

@main
func main() -> lang.i32 {
    if render(IntSrc(v: 9)) != "i" { return 10 }
    0
}
