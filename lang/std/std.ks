// Kestrel Standard Library
//
// This module re-exports the public API of the standard library.
// Import with: import std

module std

// Core types and protocols
public import std.core.ordering.(Ordering)
public import std.core.bool.(Bool, true, false)
public import std.core.nil.(Nil, nil)
public import std.core.protocols.(
    Equatable,
    Comparable,
    Hashable,
    Hasher,
    DefaultHasher,
    Cloneable,
    NonCopyable,
    Defaultable
)
public import std.core.numeric.(
    Numeric,
    Integer,
    SignedInteger,
    UnsignedInteger,
    FloatingPoint,
    Steppable
)

// Numeric types
public import std.core.int8.(Int8)
public import std.core.int16.(Int16)
public import std.core.int32.(Int32)
public import std.core.int64.(Int64, Int)
public import std.core.uint8.(UInt8)
public import std.core.uint16.(UInt16)
public import std.core.uint32.(UInt32)
public import std.core.uint64.(UInt64, UInt)
public import std.core.float32.(Float32)
public import std.core.float64.(Float64, Float)

// Operator protocols
public import std.ops.arithmetic.(
    Addable,
    Subtractable,
    Multipliable,
    Divisible,
    Modulo,
    Negatable
)
public import std.ops.comparison.(
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual
)
public import std.ops.logical.(And, Or, Not)
public import std.ops.bitwise.(
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    BitwiseNot,
    LeftShift,
    RightShift
)
public import std.ops.assign.(
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    BitwiseAndAssign,
    BitwiseOrAssign,
    BitwiseXorAssign,
    LeftShiftAssign,
    RightShiftAssign
)
public import std.ops.range.(
    RangeConstructible,
    ClosedRangeConstructible,
    Range,
    ClosedRange,
    RangeIterator,
    ClosedRangeIterator
)
public import std.ops.literals.(
    ExpressibleByBoolLiteral,
    ExpressibleByIntLiteral,
    ExpressibleByFloatLiteral,
    ExpressibleByStringLiteral,
    ExpressibleByNilLiteral,
    ExpressibleByArrayLiteral,
    ExpressibleByDictionaryLiteral
)

// Error handling
public import std.result.error.(
    Error,
    Residual,
    Tryable,
    Throwable,
    Returnable,
    Convertible
)
public import std.result.optional.(Optional, OptionalIterator)
public import std.result.result.(Result, ResultIterator)

// Iterators
public import std.iter.iterator.(
    Iterator,
    Iterable,
    Collectable,
    Functor
)
public import std.iter.adapters.(
    MapIterator,
    FilterIterator,
    FilterMapIterator,
    FlatMapIterator,
    TakeIterator,
    TakeWhileIterator,
    SkipIterator,
    SkipWhileIterator,
    StepByIterator,
    EnumerateIterator,
    ZipIterator,
    ChainIterator,
    CycleIterator,
    IntersperseIterator,
    PeekableIterator,
    FuseIterator,
    EmptyIterator,
    OnceIterator,
    RepeatIterator,
    RepeatNIterator,
    empty,
    once,
    repeat,
    repeatN
)

// FFI
public import std.ffi.(FFISafe)

// Memory
public import std.memory.layout.(Layout)
public import std.memory.allocator.(
    Allocator,
    SystemAllocator,
    GlobalAllocator,
    ArenaAllocator,
    PoolAllocator
)
public import std.memory.pointer.(
    RawPointer,
    Pointer,
    Slice,
    SliceIterator
)
public import std.memory.buffer.(Buffer, ArcBox)

// Collections
public import std.collections.array.(Array, ArrayIterator)
public import std.collections.dictionary.(
    Dictionary,
    DictionaryIterator,
    KeysView,
    KeysIterator,
    ValuesView,
    ValuesIterator
)
public import std.collections.set.(Set, SetIterator)

// Text
public import std.text.char.(Byte, CodePoint, Char, decodeUtf8)
public import std.text.string.(String, SplitIterator)
public import std.text.views.(
    BytesView,
    BytesIterator,
    CodePointsView,
    CodePointsIterator,
    CharsView,
    CharsIterator,
    LinesView,
    LinesIterator,
    ByteIndex,
    CodePointIndex,
    CharIndex
)

// Serialization
public import std.serde.serde.(
    Serialize,
    Deserialize,
    Serializer,
    Deserializer,
    ObjectSerializer,
    ArraySerializer,
    ObjectVisitor,
    ObjectAccess,
    ArrayAccess,
    SerializeError,
    DeserializeError
)

// JSON
public import std.json.json.(
    Json,
    JsonValue,
    JsonError,
    JsonSerializer,
    JsonDeserializer,
    JsonObjectSerializer,
    JsonArraySerializer,
    JsonObjectAccess
)
