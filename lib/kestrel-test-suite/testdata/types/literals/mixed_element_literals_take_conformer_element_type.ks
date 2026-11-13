// test: execution
// stdlib: true
// expect-exit: 0
//
// Elements of an array/dictionary literal whose target type is a custom conformer
// take the conformer's `Element`/`Value` type as their expected type, so mixed
// literals (`[1, "two", true, null]`) each build a J. Today the elements unify
// with each other instead ("expected integer literal got bool literal").

module Test

// Mirrors quill's Value: every JSON literal form builds a J.
enum J: Cloneable, ExpressibleByStringLiteral, ExpressibleByIntLiteral, ExpressibleByBoolLiteral,
        ExpressibleByNullLiteral, _ExpressibleByArrayLiteral, _ExpressibleByDictionaryLiteral {
    case Str(String)
    case Int(Int64)
    case Boolean(Bool)
    case Null
    case Arr(Array[J])
    case Obj(Dictionary[String, J])

    type Element = J
    type Key = String
    type Value = J

    init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) { self = .Str(String(stringLiteral: ptr, length)) }
    init(intLiteral value: lang.i64) { self = .Int(Int64(intLiteral: value)) }
    init(boolLiteral value: lang.i1) { self = .Boolean(Bool(boolLiteral: value)) }
    init() { self = .Null }
    init(consuming _arrayLiteralPointer _arrayLiteralPointer: lang.ptr[J], consuming _arrayLiteralCount _arrayLiteralCount: lang.i64) {
        self = .Arr(Array[J](_arrayLiteralPointer: _arrayLiteralPointer, _arrayLiteralCount: _arrayLiteralCount))
    }
    init(consuming _dictionaryLiteralPointer: lang.ptr[(String, J)], consuming _dictionaryLiteralCount: lang.i64) {
        self = .Obj(Dictionary[String, J](_dictionaryLiteralPointer, _dictionaryLiteralCount))
    }

    func clone() -> J {
        match self {
            .Str(s) => .Str(s.clone()),
            .Int(n) => .Int(n),
            .Boolean(b) => .Boolean(b),
            .Null => .Null,
            .Arr(a) => .Arr(a.clone()),
            .Obj(o) => .Obj(o.clone())
        }
    }
}

@main
func main() -> Int64 {
    let flat: J = ["id": 42, "admin": false, "name": "Ada", "none": null];
    guard let .Obj(o) = flat else { return 1 }
    guard let some .Boolean(b) = o("admin") else { return 2 }
    guard not b else { return 3 }
    guard let some .Null = o("none") else { return 4 }
    let list: J = [1, "two", true, null];
    guard let .Arr(a) = list else { return 5 }
    guard a.count == 4 else { return 6 }
    0
}
