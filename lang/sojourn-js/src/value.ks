module js

// JavaScript values, NaN-boxed into 64 bits.
//
// A double is stored as its own bits. Every other value hides in the NaN
// space above 0xFFF8_0000_0000_0000, which no double the engine produces
// uses (NaNs are canonicalized to 0x7FF8…): the top 16 bits are a tag and
// the low 48 bits a payload, a heap index for cells.

/// Value tags: the top 16 bits of a boxed value.
struct Tag {
    static var undefined: UInt64 { 0xFFF9 }
    static var nullValue: UInt64 { 0xFFFA }
    static var bool: UInt64 { 0xFFFB }
    /// No value: an uninitialized `let` (the temporal dead zone) or an
    /// array hole.
    static var empty: UInt64 { 0xFFFC }
    static var object: UInt64 { 0xFFFD }
    static var string: UInt64 { 0xFFFE }
    static var symbol: UInt64 { 0xFFFF }
}

/// The heap index of a cell.
type Ref = Int64;

struct Value {
    var bits: UInt64

    static func boxed(tag: UInt64, payload: Int64) -> Value {
        Value(bits: (tag << 48) | (UInt64(bitPattern: payload) & 0xFFFFFFFFFFFF))
    }

    static func number(n: Float64) -> Value {
        if n.isNaN { return Value(bits: 0x7FF8000000000000); }
        Value(bits: n.bitPattern)
    }

    static func int(n: Int64) -> Value { Value.number(Float64(from: n)) }
    static func bool(b: Bool) -> Value { Value.boxed(Tag.bool, if b { 1 } else { 0 }) }
    static func object(r: Ref) -> Value { Value.boxed(Tag.object, r) }
    static func string(r: Ref) -> Value { Value.boxed(Tag.string, r) }
    static func symbol(r: Ref) -> Value { Value.boxed(Tag.symbol, r) }
    static var undefined: Value { Value.boxed(Tag.undefined, 0) }
    static var nullValue: Value { Value.boxed(Tag.nullValue, 0) }
    static var empty: Value { Value.boxed(Tag.empty, 0) }

    var tag: UInt64 { self.bits >> 48 }
    var payload: Int64 { Int64(bitPattern: self.bits & 0xFFFFFFFFFFFF) }

    var isNumber: Bool { self.tag < Tag.undefined }
    var isUndefined: Bool { self.tag == Tag.undefined }
    var isNull: Bool { self.tag == Tag.nullValue }
    var isNullish: Bool { self.tag == Tag.undefined or self.tag == Tag.nullValue }
    var isBool: Bool { self.tag == Tag.bool }
    var isEmpty: Bool { self.tag == Tag.empty }
    var isObject: Bool { self.tag == Tag.object }
    var isString: Bool { self.tag == Tag.string }
    var isSymbol: Bool { self.tag == Tag.symbol }
    /// The value refers to a heap cell, which the GC must trace.
    var isCell: Bool { self.tag >= Tag.object }

    var asNumber: Float64 { Float64(bitPattern: self.bits) }
    var asBool: Bool { self.payload != 0 }
    var asRef: Ref { self.payload }

    func same(other: Value) -> Bool { self.bits == other.bits }
}
