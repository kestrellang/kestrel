// test: execution
// stdlib: true
// expect-exit: 52
//
// A type conforming to both `ExpressibleByStringLiteral` and
// `_ExpressibleByArrayLiteral` must build array literals with the array init.
// Today `resolve_array_literal_init` (kestrel-mir-lower body/literal.rs) picks the
// first init shaped (mut self, Pointer(_), i64), which also matches
// `init(stringLiteral:length:)` — the literal silently becomes `.Str` (exit 10).
// Expected: `.Arr` with 2 elements (exit 52).

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
    let x: J = [J.Int(1), J.Int(2)];
    match x {
        .Str(_) => 10,
        .Int(n) => 20 + n,
        .Boolean(_) => 30,
        .Null => 40,
        .Arr(a) => 50 + a.count,
        .Obj(o) => 60 + o.count
    }
}
