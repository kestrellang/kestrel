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
    func serializeArray[S](values: Array[S]) -> Result[(), Error] where S: Serialize
    func serializeMap[K, V](entries: Array[(K, V)]) -> Result[(), Error] where K: Serialize, V: Serialize

    // Struct/object serialization
    func beginObject(name: String, fieldCount: Int) -> Result[ObjectSerializer, Error]
    func beginArray(length: Int) -> Result[ArraySerializer, Error]

    // Get the final output
    func finish() -> Result[Output, Error]
}

// ObjectSerializer - writes struct/object fields
public protocol ObjectSerializer {
    type Error: Error

    func serializeField[V](name: String, value: V) -> Result[(), Error] where V: Serialize
    func end() -> Result[(), Error]
}

// ArraySerializer - writes array elements
public protocol ArraySerializer {
    type Error: Error

    func serializeElement[V](value: V) -> Result[(), Error] where V: Serialize
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
//     public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
//         let obj = try serializer.beginObject(name: "Point", fieldCount: 2)
//         try obj.serializeField(name: "x", value: self.x)
//         try obj.serializeField(name: "y", value: self.y)
//         obj.end()
//     }
// }
// ```
public protocol Serialize {
    func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer
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
    func deserializeArray[T]() -> Result[Array[T], Error] where T: Deserialize
    func deserializeMap[K, V]() -> Result[Dictionary[K, V], Error] where K: Deserialize, K: Hashable, V: Deserialize

    // Struct/object deserialization
    func deserializeObject[V](visitor: V.Visitor) -> Result[V, Error] where V: Deserialize
}

// Deserialize - types that can be deserialized from any format
//
// Implement this protocol to make a type deserializable:
//
// ```kestrel
// public struct Point: Deserialize {
//     type Visitor = PointVisitor
//
//     public static func deserialize[D](from deserializer: mutating D) -> Result[Point, D.Error] {
//         deserializer.deserializeObject(visitor: PointVisitor())
//     }
// }
//
// public struct PointVisitor: ObjectVisitor {
//     type Value = Point
//
//     public func visit[A](access: mutating A) where A: ObjectAccess -> Result[Point, A.Error] {
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
    type Visitor: ObjectVisitor where Visitor.Value = Self
    static func deserialize[D](from deserializer: mutating D) -> Result[Self, D.Error] where D: Deserializer
}

// ObjectVisitor - visits object fields during deserialization
public protocol ObjectVisitor {
    type Value

    func visit[A](access: mutating A) -> Result[Value, A.Error] where A: ObjectAccess
}

// ObjectAccess - provides access to object fields
public protocol ObjectAccess {
    type Error: Error

    func nextField() -> Result[Optional[String], Error]
    func value[V]() -> Result[V, Error] where V: Deserialize
    func skipValue() -> Result[(), Error]
}

// ArrayAccess - provides access to array elements
public protocol ArrayAccess {
    type Error: Error

    func hasNext() -> Bool
    func next[V]() -> Result[V, Error] where V: Deserialize
}

// Default implementations for primitives

extension Bool: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeBool(value: self)
    }
}

extension Int: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeInt(value: self)
    }
}

extension Int8: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeInt8(value: self)
    }
}

extension Int16: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeInt16(value: self)
    }
}

extension Int32: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeInt32(value: self)
    }
}

extension Int64: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeInt64(value: self)
    }
}

extension UInt: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeUInt(value: self)
    }
}

extension UInt8: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeUInt8(value: self)
    }
}

extension UInt16: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeUInt16(value: self)
    }
}

extension UInt32: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeUInt32(value: self)
    }
}

extension UInt64: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeUInt64(value: self)
    }
}

extension Float32: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeFloat32(value: self)
    }
}

extension Float64: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeFloat64(value: self)
    }
}

extension String: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        serializer.serializeString(value: self)
    }
}

extension Optional[T]: Serialize where T: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        match self {
            .Some(let value) => value.serialize(to: serializer),
            .None => serializer.serializeNil()
        }
    }
}

extension Array[T]: Serialize where T: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
        let arr = try serializer.beginArray(length: self.count)
        /* for item in self {
            try arr.serializeElement(value: item)
        } */
        arr.end()
    }
}

extension Dictionary[K, V]: Serialize where K: Serialize, V: Serialize {
    public func serialize[S](to serializer: mutating S) -> Result[(), S.Error] where S: Serializer {
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

    public static func deserialize[D](from deserializer: mutating D) -> Result[Bool, D.Error] where D: Deserializer {
        deserializer.deserializeBool()
    }
}

public struct BoolVisitor: ObjectVisitor {
    type Value = Bool

    public func visit[A](access: mutating A) -> Result[Bool, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "Bool cannot be deserialized from object"))
    }
}

extension Int: Deserialize {
    type Visitor = IntVisitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[Int, D.Error] where D: Deserializer {
        deserializer.deserializeInt()
    }
}

public struct IntVisitor: ObjectVisitor {
    type Value = Int

    public func visit[A](access: mutating A) -> Result[Int, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "Int cannot be deserialized from object"))
    }
}

extension Int8: Deserialize {
    type Visitor = Int8Visitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[Int8, D.Error] where D: Deserializer {
        deserializer.deserializeInt8()
    }
}

public struct Int8Visitor: ObjectVisitor {
    type Value = Int8

    public func visit[A](access: mutating A) -> Result[Int8, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "Int8 cannot be deserialized from object"))
    }
}

extension Int16: Deserialize {
    type Visitor = Int16Visitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[Int16, D.Error] where D: Deserializer {
        deserializer.deserializeInt16()
    }
}

public struct Int16Visitor: ObjectVisitor {
    type Value = Int16

    public func visit[A](access: mutating A) -> Result[Int16, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "Int16 cannot be deserialized from object"))
    }
}

extension Int32: Deserialize {
    type Visitor = Int32Visitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[Int32, D.Error] where D: Deserializer {
        deserializer.deserializeInt32()
    }
}

public struct Int32Visitor: ObjectVisitor {
    type Value = Int32

    public func visit[A](access: mutating A) -> Result[Int32, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "Int32 cannot be deserialized from object"))
    }
}

extension Int64: Deserialize {
    type Visitor = Int64Visitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[Int64, D.Error] where D: Deserializer {
        deserializer.deserializeInt64()
    }
}

public struct Int64Visitor: ObjectVisitor {
    type Value = Int64

    public func visit[A](access: mutating A) -> Result[Int64, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "Int64 cannot be deserialized from object"))
    }
}

extension UInt: Deserialize {
    type Visitor = UIntVisitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[UInt, D.Error] where D: Deserializer {
        deserializer.deserializeUInt()
    }
}

public struct UIntVisitor: ObjectVisitor {
    type Value = UInt

    public func visit[A](access: mutating A) -> Result[UInt, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "UInt cannot be deserialized from object"))
    }
}

extension UInt8: Deserialize {
    type Visitor = UInt8Visitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[UInt8, D.Error] where D: Deserializer {
        deserializer.deserializeUInt8()
    }
}

public struct UInt8Visitor: ObjectVisitor {
    type Value = UInt8

    public func visit[A](access: mutating A) -> Result[UInt8, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "UInt8 cannot be deserialized from object"))
    }
}

extension UInt16: Deserialize {
    type Visitor = UInt16Visitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[UInt16, D.Error] where D: Deserializer {
        deserializer.deserializeUInt16()
    }
}

public struct UInt16Visitor: ObjectVisitor {
    type Value = UInt16

    public func visit[A](access: mutating A) -> Result[UInt16, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "UInt16 cannot be deserialized from object"))
    }
}

extension UInt32: Deserialize {
    type Visitor = UInt32Visitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[UInt32, D.Error] where D: Deserializer {
        deserializer.deserializeUInt32()
    }
}

public struct UInt32Visitor: ObjectVisitor {
    type Value = UInt32

    public func visit[A](access: mutating A) -> Result[UInt32, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "UInt32 cannot be deserialized from object"))
    }
}

extension UInt64: Deserialize {
    type Visitor = UInt64Visitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[UInt64, D.Error] where D: Deserializer {
        deserializer.deserializeUInt64()
    }
}

public struct UInt64Visitor: ObjectVisitor {
    type Value = UInt64

    public func visit[A](access: mutating A) -> Result[UInt64, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "UInt64 cannot be deserialized from object"))
    }
}

extension Float32: Deserialize {
    type Visitor = Float32Visitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[Float32, D.Error] where D: Deserializer {
        deserializer.deserializeFloat32()
    }
}

public struct Float32Visitor: ObjectVisitor {
    type Value = Float32

    public func visit[A](access: mutating A) -> Result[Float32, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "Float32 cannot be deserialized from object"))
    }
}

extension Float64: Deserialize {
    type Visitor = Float64Visitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[Float64, D.Error] where D: Deserializer {
        deserializer.deserializeFloat64()
    }
}

public struct Float64Visitor: ObjectVisitor {
    type Value = Float64

    public func visit[A](access: mutating A) -> Result[Float64, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "Float64 cannot be deserialized from object"))
    }
}

extension String: Deserialize {
    type Visitor = StringVisitor

    public static func deserialize[D](from deserializer: mutating D) -> Result[String, D.Error] where D: Deserializer {
        deserializer.deserializeString()
    }
}

public struct StringVisitor: ObjectVisitor {
    type Value = String

    public func visit[A](access: mutating A) -> Result[String, A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "String cannot be deserialized from object"))
    }
}

extension Optional[T]: Deserialize where T: Deserialize {
    type Visitor = OptionalVisitor[T]

    public static func deserialize[D](from deserializer: mutating D) -> Result[Optional[T], D.Error] where D: Deserializer {
        // Deserializers should handle null -> None, otherwise deserialize T
        // This is a simplified version; full impl would peek at next token
        match T.deserialize(from: deserializer) {
            .Ok(let value) => .Ok(.Some(value)),
            .Err(_) => .Ok(.None)
        }
    }
}

public struct OptionalVisitor[T]: ObjectVisitor where T: Deserialize {
    type Value = Optional[T]

    public func visit[A](access: mutating A) -> Result[Optional[T], A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "Optional cannot be deserialized from object"))
    }
}

extension Array[T]: Deserialize where T: Deserialize {
    type Visitor = ArrayVisitor[T]

    public static func deserialize[D](from deserializer: mutating D) -> Result[Array[T], D.Error] where D: Deserializer {
        deserializer.deserializeArray[T]()
    }
}

public struct ArrayVisitor[T]: ObjectVisitor where T: Deserialize {
    type Value = Array[T]

    public func visit[A](access: mutating A) -> Result[Array[T], A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "Array cannot be deserialized from object"))
    }
}

extension Dictionary[K, V]: Deserialize where K: Deserialize, K: Hashable, V: Deserialize {
    type Visitor = DictionaryVisitor[K, V]

    public static func deserialize[D](from deserializer: mutating D) -> Result[Dictionary[K, V], D.Error] where D: Deserializer {
        deserializer.deserializeMap[K, V]()
    }
}

public struct DictionaryVisitor[K, V]: ObjectVisitor where K: Deserialize, K: Hashable, V: Deserialize {
    type Value = Dictionary[K, V]

    public func visit[A](access: mutating A) -> Result[Dictionary[K, V], A.Error] where A: ObjectAccess {
        .Err(A.Error(message: "Dictionary cannot be deserialized from object"))
    }
}
