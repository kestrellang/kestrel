// test: execution
// stdlib: true
// expect-exit: 0
//
// An array literal's element type is `_ExpressibleByArrayLiteral.Element` —
// the tagged associated type — not whichever `Element` the target type binds
// first. `Bag` also conforms to `Shelf`, whose own `Element` is `String`.

module Test

protocol Shelf {
    type Element
    func label() -> Element
}

struct Bag: _ExpressibleByArrayLiteral, Shelf {
    var items: Array[Int64]

    type Shelf.Element = String
    type _ExpressibleByArrayLiteral.Element = Int64

    init(consuming _arrayLiteralPointer _arrayLiteralPointer: lang.ptr[Int64], consuming _arrayLiteralCount _arrayLiteralCount: lang.i64) {
        self.items = Array[Int64](_arrayLiteralPointer: _arrayLiteralPointer, _arrayLiteralCount: _arrayLiteralCount)
    }

    func label() -> String { "bag" }
}

@main
func main() -> Int64 {
    let b: Bag = [1, 2, 3];
    if b.items.count != 3 { return 1 }
    if b.items(1) != 2 { return 2 }
    0
}
