// test: execution
// stdlib: true
// include: _cross_module_field_subscript_user.ks
// expect-exit: 0

// Regression: `obj.arrayField(unchecked: i)` where `obj`'s struct lives in a
// module MIR-lowered after the caller's module. `resolve_field_type` used to
// return `MirTy::unit()` in that case, propagating unit as the subscript
// call's self_type and producing a phantom `...S_v` mangled symbol that
// codegen couldn't resolve ("call to undeclared function").
//
// The caller (UserSide.firstItem) lives in the include file, which is added
// to the ECS before this file — so its body is lowered first, while `Bag`
// from this module isn't in `module.structs` yet.

module Test

import std.text.(String)
import std.collections.(Array)
import UserSide.(firstItem)

public struct Bag: Cloneable {
    public var items: Array[String]

    public init() {
        self.items = Array[String]()
    }

    public func clone() -> Bag {
        var b = Bag();
        b.items = self.items.clone();
        b
    }
}

func main() -> lang.i64 {
    var bag = Bag();
    bag.items.append("hello");
    let got = firstItem(bag);
    if got.isEqual(to: "hello") { 0 } else { 1 }
}
