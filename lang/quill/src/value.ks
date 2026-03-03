// Quill Value - format-agnostic intermediate representation

module quill.value


// ============================================================================
// VALUE ENUM
// ============================================================================

/// A format-agnostic data value used as an intermediate representation
/// between user types and serialization formats (JSON, TOML, etc.).
public enum Value: Cloneable {
    /// A null/missing value.
    case Null

    /// A boolean value.
    case Boolean(std.core.Bool)

    /// An integer value.
    case Int(Int64)

    /// A floating-point value.
    case Float(Float64)

    /// A string value.
    case Str(String)

    /// An ordered array of values.
    case Arr(std.collections.Array[Value])

    /// A string-keyed mapping of values.
    case Obj(Dictionary[String, Value])
}

// ============================================================================
// VALUE METHODS
// ============================================================================

extend Value {
    public func clone() -> Value {
        match self {
            .Null => .Null,
            .Boolean(b) => .Boolean(b),
            .Int(n) => .Int(n),
            .Float(f) => .Float(f),
            .Str(s) => .Str(s.clone()),
            .Arr(arr) => .Arr(arr.clone()),
            .Obj(obj) => .Obj(obj.clone())
        }
    }

    /// Returns true if this value is Null.
    public func isNull() -> Bool {
        match self {
            .Null => true,
            _ => false
        }
    }

    /// Returns the boolean value, or None if not a Boolean.
    public func asBool() -> Optional[Bool] {
        match self {
            .Boolean(b) => .Some(b),
            _ => .None
        }
    }

    /// Returns the integer value, or None if not an Int.
    public func asInt() -> Optional[Int64] {
        match self {
            .Int(n) => .Some(n),
            _ => .None
        }
    }

    /// Returns the float value, or None if not a Float.
    public func asFloat() -> Optional[Float64] {
        match self {
            .Float(f) => .Some(f),
            _ => .None
        }
    }

    /// Returns the string value, or None if not a Str.
    public func asString() -> Optional[String] {
        match self {
            .Str(s) => .Some(s),
            _ => .None
        }
    }

    /// Returns the array value, or None if not an Arr.
    public func asArray() -> Optional[Array[Value]] {
        match self {
            .Arr(arr) => .Some(arr),
            _ => .None
        }
    }

    /// Returns the object value, or None if not an Obj.
    public func asObject() -> Optional[Dictionary[String, Value]] {
        match self {
            .Obj(obj) => .Some(obj),
            _ => .None
        }
    }

    /// Looks up a key in this value if it is an Obj.
    /// Returns the value for the key, or None if not found or not an Obj.
    public func value(forKey key: String) -> Optional[Value] {
        match self {
            .Obj(obj) => obj(key),
            _ => .None
        }
    }

    /// Returns the name of the value's type for error messages.
    public func typeName() -> String {
        match self {
            .Null => "null",
            .Boolean(_) => "bool",
            .Int(_) => "int",
            .Float(_) => "float",
            .Str(_) => "string",
            .Arr(_) => "array",
            .Obj(_) => "object"
        }
    }
}
