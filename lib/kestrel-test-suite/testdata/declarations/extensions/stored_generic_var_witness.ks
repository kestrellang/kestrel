// test: execution
// stdlib: true
// expect-exit: 0
//
// A stored var on a GENERIC conformer witnessing an associated-typed protocol
// property (`var item: Item` with `type Item = T`), dispatched through a type
// parameter. Two parts had to work together:
//   1. conformance checking must equate `var item: T` with `var item: Item`
//      via the `type Item = T` binding (was a false E456 "wrong type");
//   2. witness lowering must synthesize the accessor carrying the conformer's
//      type params + bind it with the conformer's type args, so mono substitutes
//      `T` per instantiation (was "does not implement 'item'").

module Test

protocol Boxy {
    type Item
    var item: Item { get set }
}

struct Box[T]: Boxy {
    type Item = T
    var item: T;
}

// two type params, witnessed field is NOT the first param — pins field-type
// substitution through the conformer's full param list.
struct Pair[A, B]: Boxy {
    type Item = B
    var first: A;
    var item: B;
}

func itemOf[X](x: X) -> X.Item where X: Boxy { x.item }            // generic getter
func setItem[X](x: X, v: X.Item) -> X.Item where X: Boxy {          // generic setter
    var u = x;
    u.item = v;
    u.item
}

@main
func main() -> lang.i32 {
    if itemOf(Box(item: 9)) != 9 { return 1 }
    if setItem(Box(item: 1), 42) != 42 { return 2 }
    if itemOf(Pair(first: 7, item: 5)) != 5 { return 3 }   // B-typed field
    if setItem(Pair(first: 7, item: 5), 8) != 8 { return 4 }
    0
}
