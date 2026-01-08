// Serialization and Deserialization protocols
//
// This module provides a generic framework for converting values to and from
// various data formats (JSON, binary, etc.).

module std.serde

// SerializeError - error during serialization
public struct SerializeError: Error {
    public var message: String

    public init(message: String) {
        self.message = message;
    }

    public var description: String {
        "SerializeError: " + self.message
    }
}

// DeserializeError - error during deserialization
public struct DeserializeError: Error {
    public var message: String
    public var position: Optional[Int]

    public init(message: String) {
        self.message = message;
        self.position = .None
    }

    public init(message: String, at position: Int) {
        self.message = message;
        self.position = .Some(position)
    }

    public var description: String {
        match self.position {
            .Some(let pos) => "DeserializeError at position " + pos.toString() + ": " + self.message,
            .None => "DeserializeError: " + self.message
        }
    }
}

// Serializer - a format-specific serializer that values can write to
//
// Each format (JSON, binary, etc.) implements this protocol.
// The serializer handles the encoding details for that format.
public protocol Serializer {
    type Output
    type Error: Error

    // Primitive serialization
    func serializeNil() -> Result[(), Error]
    func serializeBool(value: Bool) -> Result[(), Error]
    func serializeInt(value: Int) -> Result[(), Error]
    func serializeInt8(value: Int8) -> Result[(), Error]
    func serializeInt16(value: Int16) -> Result[(), Error]
    func serializeInt32(value: Int32) -> Result[(), Error]
    func serializeInt64(value: Int64) -> Result[(), Error]
    func serializeUInt(value: UInt) -> Result[(), Error]
    func serializeUInt8(value: UInt8) -> Result[(), Error]
    func serializeUInt16(value: UInt16) -> Result[(), Error]
    func serializeUInt32(value: UInt32) -> Result[(), Error]
    func serializeUInt64(value: UInt64) -> Result[(), Error]
    func serializeFloat32(value: Float32) -> Result[(), Error]
    func serializeFloat64(value: Float64) -> Result[(), Error]
    func serializeString(value: String) -> Result[(), Error]

    // Compound types
    func serializeArray[S: Serialize](values: Array[S]) -> Result[(), Error]
    func serializeMap[K: Serialize, V: Serialize](entries: Array[(K, V)]) -> Result[(), Error]

    // Struct/object serialization
    func beginObject(name: String, fieldCount: Int) -> Result[ObjectSerializer, Error]
    func beginArray(length: Int) -> Result[ArraySerializer, Error]

    // Get the final output
    func finish() -> Result[Output, Error]
}

// ObjectSerializer - writes struct/object fields
public protocol ObjectSerializer {
    type Error: Error

    func serializeField[V: Serialize](name: String, value: V) -> Result[(), Error]
    func end() -> Result[(), Error]
}

// ArraySerializer - writes array elements
public protocol ArraySerializer {
    type Error: Error

    func serializeElement[V: Serialize](value: V) -> Result[(), Error]
    func end() -> Result[(), Error]
}

// Serialize - types that can be serialized to any format
//
// Implement this protocol to make a type serializable:
//
// ```kestrel
// public struct Point: Serialize {
//     var x: Int
//     var y: Int
//
//     public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
//         let obj = try serializer.beginObject(name: "Point", fieldCount: 2)
//         try obj.serializeField(name: "x", value: self.x)
//         try obj.serializeField(name: "y", value: self.y)
//         obj.end()
//     }
// }
// ```
public protocol Serialize {
    func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error]
}

// Deserializer - a format-specific deserializer that produces values
//
// Each format (JSON, binary, etc.) implements this protocol.
// The deserializer handles the decoding details for that format.
public protocol Deserializer {
    type Error: Error

    // Primitive deserialization
    func deserializeNil() -> Result[Nil, Error]
    func deserializeBool() -> Result[Bool, Error]
    func deserializeInt() -> Result[Int, Error]
    func deserializeInt8() -> Result[Int8, Error]
    func deserializeInt16() -> Result[Int16, Error]
    func deserializeInt32() -> Result[Int32, Error]
    func deserializeInt64() -> Result[Int64, Error]
    func deserializeUInt() -> Result[UInt, Error]
    func deserializeUInt8() -> Result[UInt8, Error]
    func deserializeUInt16() -> Result[UInt16, Error]
    func deserializeUInt32() -> Result[UInt32, Error]
    func deserializeUInt64() -> Result[UInt64, Error]
    func deserializeFloat32() -> Result[Float32, Error]
    func deserializeFloat64() -> Result[Float64, Error]
    func deserializeString() -> Result[String, Error]

    // Compound types
    func deserializeArray[T: Deserialize]() -> Result[Array[T], Error]
    func deserializeMap[K: Deserialize + Hashable, V: Deserialize]() -> Result[Dictionary[K, V], Error]

    // Struct/object deserialization
    func deserializeObject[V: Deserialize](visitor: V.Visitor) -> Result[V, Error]
}

// Deserialize - types that can be deserialized from any format
//
// Implement this protocol to make a type deserializable:
//
// ```kestrel
// public struct Point: Deserialize {
//     type Visitor = PointVisitor
//
//     public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Point, D.Error] {
//         deserializer.deserializeObject(visitor: PointVisitor())
//     }
// }
//
// public struct PointVisitor: ObjectVisitor {
//     type Value = Point
//
//     public func visit[A: ObjectAccess](access: ref A) -> Result[Point, A.Error] {
//         var x: Optional[Int] = .None
//         var y: Optional[Int] = .None
//         while let field = try access.nextField() {
//             match field {
//                 "x" => x = .Some(try access.value[Int]()),
//                 "y" => y = .Some(try access.value[Int]()),
//                 _ => try access.skipValue()
//             }
//         }
//         .Ok(Point(x: x.unwrap(), y: y.unwrap()))
//     }
// }
// ```
public protocol Deserialize {
    type Visitor: ObjectVisitor where Visitor.Value == Self
    static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Self, D.Error]
}

// ObjectVisitor - visits object fields during deserialization
public protocol ObjectVisitor {
    type Value

    func visit[A: ObjectAccess](access: ref A) -> Result[Value, A.Error]
}

// ObjectAccess - provides access to object fields
public protocol ObjectAccess {
    type Error: Error

    func nextField() -> Result[Optional[String], Error]
    func value[V: Deserialize]() -> Result[V, Error]
    func skipValue() -> Result[(), Error]
}

// ArrayAccess - provides access to array elements
public protocol ArrayAccess {
    type Error: Error

    func hasNext() -> Bool
    func next[V: Deserialize]() -> Result[V, Error]
}

// Default implementations for primitives

extension Bool: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeBool(value: self)
    }
}

extension Int: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeInt(value: self)
    }
}

extension Int8: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeInt8(value: self)
    }
}

extension Int16: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeInt16(value: self)
    }
}

extension Int32: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeInt32(value: self)
    }
}

extension Int64: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeInt64(value: self)
    }
}

extension UInt: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeUInt(value: self)
    }
}

extension UInt8: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeUInt8(value: self)
    }
}

extension UInt16: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeUInt16(value: self)
    }
}

extension UInt32: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeUInt32(value: self)
    }
}

extension UInt64: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeUInt64(value: self)
    }
}

extension Float32: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeFloat32(value: self)
    }
}

extension Float64: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeFloat64(value: self)
    }
}

extension String: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        serializer.serializeString(value: self)
    }
}

extension Optional[T]: Serialize where T: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        match self {
            .Some(let value) => value.serialize(to: serializer),
            .None => serializer.serializeNil()
        }
    }
}

extension Array[T]: Serialize where T: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        let arr = try serializer.beginArray(length: self.count)
        /* for item in self {
            try arr.serializeElement(value: item)
        } */
        arr.end()
    }
}

extension Dictionary[K, V]: Serialize where K: Serialize, V: Serialize {
    public func serialize[S: Serializer](to serializer: ref S) -> Result[(), S.Error] {
        var entries: Array[(K, V)] = []
        /* for (key, value) in self {
            entries.append((key, value))
        } */
        serializer.serializeMap(entries: entries)
    }
}

// Default Deserialize implementations for primitives

extension Bool: Deserialize {
    type Visitor = BoolVisitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Bool, D.Error] {
        deserializer.deserializeBool()
    }
}

public struct BoolVisitor: ObjectVisitor {
    type Value = Bool

    public func visit[A: ObjectAccess](access: ref A) -> Result[Bool, A.Error] {
        .Err(A.Error(message: "Bool cannot be deserialized from object"))
    }
}

extension Int: Deserialize {
    type Visitor = IntVisitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Int, D.Error] {
        deserializer.deserializeInt()
    }
}

public struct IntVisitor: ObjectVisitor {
    type Value = Int

    public func visit[A: ObjectAccess](access: ref A) -> Result[Int, A.Error] {
        .Err(A.Error(message: "Int cannot be deserialized from object"))
    }
}

extension Int8: Deserialize {
    type Visitor = Int8Visitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Int8, D.Error] {
        deserializer.deserializeInt8()
    }
}

public struct Int8Visitor: ObjectVisitor {
    type Value = Int8

    public func visit[A: ObjectAccess](access: ref A) -> Result[Int8, A.Error] {
        .Err(A.Error(message: "Int8 cannot be deserialized from object"))
    }
}

extension Int16: Deserialize {
    type Visitor = Int16Visitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Int16, D.Error] {
        deserializer.deserializeInt16()
    }
}

public struct Int16Visitor: ObjectVisitor {
    type Value = Int16

    public func visit[A: ObjectAccess](access: ref A) -> Result[Int16, A.Error] {
        .Err(A.Error(message: "Int16 cannot be deserialized from object"))
    }
}

extension Int32: Deserialize {
    type Visitor = Int32Visitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Int32, D.Error] {
        deserializer.deserializeInt32()
    }
}

public struct Int32Visitor: ObjectVisitor {
    type Value = Int32

    public func visit[A: ObjectAccess](access: ref A) -> Result[Int32, A.Error] {
        .Err(A.Error(message: "Int32 cannot be deserialized from object"))
    }
}

extension Int64: Deserialize {
    type Visitor = Int64Visitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Int64, D.Error] {
        deserializer.deserializeInt64()
    }
}

public struct Int64Visitor: ObjectVisitor {
    type Value = Int64

    public func visit[A: ObjectAccess](access: ref A) -> Result[Int64, A.Error] {
        .Err(A.Error(message: "Int64 cannot be deserialized from object"))
    }
}

extension UInt: Deserialize {
    type Visitor = UIntVisitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[UInt, D.Error] {
        deserializer.deserializeUInt()
    }
}

public struct UIntVisitor: ObjectVisitor {
    type Value = UInt

    public func visit[A: ObjectAccess](access: ref A) -> Result[UInt, A.Error] {
        .Err(A.Error(message: "UInt cannot be deserialized from object"))
    }
}

extension UInt8: Deserialize {
    type Visitor = UInt8Visitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[UInt8, D.Error] {
        deserializer.deserializeUInt8()
    }
}

public struct UInt8Visitor: ObjectVisitor {
    type Value = UInt8

    public func visit[A: ObjectAccess](access: ref A) -> Result[UInt8, A.Error] {
        .Err(A.Error(message: "UInt8 cannot be deserialized from object"))
    }
}

extension UInt16: Deserialize {
    type Visitor = UInt16Visitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[UInt16, D.Error] {
        deserializer.deserializeUInt16()
    }
}

public struct UInt16Visitor: ObjectVisitor {
    type Value = UInt16

    public func visit[A: ObjectAccess](access: ref A) -> Result[UInt16, A.Error] {
        .Err(A.Error(message: "UInt16 cannot be deserialized from object"))
    }
}

extension UInt32: Deserialize {
    type Visitor = UInt32Visitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[UInt32, D.Error] {
        deserializer.deserializeUInt32()
    }
}

public struct UInt32Visitor: ObjectVisitor {
    type Value = UInt32

    public func visit[A: ObjectAccess](access: ref A) -> Result[UInt32, A.Error] {
        .Err(A.Error(message: "UInt32 cannot be deserialized from object"))
    }
}

extension UInt64: Deserialize {
    type Visitor = UInt64Visitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[UInt64, D.Error] {
        deserializer.deserializeUInt64()
    }
}

public struct UInt64Visitor: ObjectVisitor {
    type Value = UInt64

    public func visit[A: ObjectAccess](access: ref A) -> Result[UInt64, A.Error] {
        .Err(A.Error(message: "UInt64 cannot be deserialized from object"))
    }
}

extension Float32: Deserialize {
    type Visitor = Float32Visitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Float32, D.Error] {
        deserializer.deserializeFloat32()
    }
}

public struct Float32Visitor: ObjectVisitor {
    type Value = Float32

    public func visit[A: ObjectAccess](access: ref A) -> Result[Float32, A.Error] {
        .Err(A.Error(message: "Float32 cannot be deserialized from object"))
    }
}

extension Float64: Deserialize {
    type Visitor = Float64Visitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Float64, D.Error] {
        deserializer.deserializeFloat64()
    }
}

public struct Float64Visitor: ObjectVisitor {
    type Value = Float64

    public func visit[A: ObjectAccess](access: ref A) -> Result[Float64, A.Error] {
        .Err(A.Error(message: "Float64 cannot be deserialized from object"))
    }
}

extension String: Deserialize {
    type Visitor = StringVisitor

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[String, D.Error] {
        deserializer.deserializeString()
    }
}

public struct StringVisitor: ObjectVisitor {
    type Value = String

    public func visit[A: ObjectAccess](access: ref A) -> Result[String, A.Error] {
        .Err(A.Error(message: "String cannot be deserialized from object"))
    }
}

extension Optional[T]: Deserialize where T: Deserialize {
    type Visitor = OptionalVisitor[T]

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Optional[T], D.Error] {
        // Deserializers should handle null -> None, otherwise deserialize T
        // This is a simplified version; full impl would peek at next token
        match T.deserialize(from: deserializer) {
            .Ok(let value) => .Ok(.Some(value)),
            .Err(_) => .Ok(.None)
        }
    }
}

public struct OptionalVisitor[T: Deserialize]: ObjectVisitor {
    type Value = Optional[T]

    public func visit[A: ObjectAccess](access: ref A) -> Result[Optional[T], A.Error] {
        .Err(A.Error(message: "Optional cannot be deserialized from object"))
    }
}

extension Array[T]: Deserialize where T: Deserialize {
    type Visitor = ArrayVisitor[T]

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Array[T], D.Error] {
        deserializer.deserializeArray[T]()
    }
}

public struct ArrayVisitor[T: Deserialize]: ObjectVisitor {
    type Value = Array[T]

    public func visit[A: ObjectAccess](access: ref A) -> Result[Array[T], A.Error] {
        .Err(A.Error(message: "Array cannot be deserialized from object"))
    }
}

extension Dictionary[K, V]: Deserialize where K: Deserialize + Hashable, V: Deserialize {
    type Visitor = DictionaryVisitor[K, V]

    public static func deserialize[D: Deserializer](from deserializer: ref D) -> Result[Dictionary[K, V], D.Error] {
        deserializer.deserializeMap[K, V]()
    }
}

public struct DictionaryVisitor[K: Deserialize + Hashable, V: Deserialize]: ObjectVisitor {
    type Value = Dictionary[K, V]

    public func visit[A: ObjectAccess](access: ref A) -> Result[Dictionary[K, V], A.Error] {
        .Err(A.Error(message: "Dictionary cannot be deserialized from object"))
    }
}
