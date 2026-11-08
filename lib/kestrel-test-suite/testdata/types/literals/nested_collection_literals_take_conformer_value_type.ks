// test: execution
// stdlib: true
// expect-exit: 0
//
// JSON-style nested literals: an inner dictionary/array literal in value position
// takes the outer conformer's `Value` type (J), recursively.

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
    let token = "abc";
    let body: J = [
        "token": .Str(token),
        "user": ["id": 42, "admin": false, "folderId": null, "tags": ["work", "draft"]]
    ];
    guard let .Obj(top) = body else { return 1 }
    guard let some .Str(t) = top("token") else { return 2 }
    guard t == "abc" else { return 3 }
    guard let some .Obj(user) = top("user") else { return 4 }
    guard let some .Int(id) = user("id") else { return 5 }
    guard id == 42 else { return 6 }
    guard let some .Null = user("folderId") else { return 7 }
    guard let some .Arr(tags) = user("tags") else { return 8 }
    guard tags.count == 2 else { return 9 }
    0
}
