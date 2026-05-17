/// Format-agnostic intermediate representation for structured data.
///
/// `Value` is the pivot type that sits between user types and wire
/// formats. Serialization converts a user type into a `Value` tree;
/// a `Format` implementation (JSON, TOML, …) then encodes the tree
/// to a string. Deserialization works in reverse.
///
/// # Examples
///
/// ```
/// let v = Value.Obj(
///     ["name": Value.Str("Alice"), "age": Value.Int(30)]
/// );
/// v.value(forKey: "name");  // Some(.Str("Alice"))
/// v.typeName();             // "object"
/// ```

module quill.value


// ============================================================================
// VALUE ENUM
// ============================================================================

/// A format-agnostic data value used as an intermediate representation
/// between user types and serialization formats (JSON, TOML, etc.).
///
/// Every node in a serialized document maps to one of these seven
/// cases. `Arr` and `Obj` nest recursively, so an entire document is
/// a single `Value` tree.
///
/// # Examples
///
/// ```
/// let s = Value.Str("hello");
/// let n = Value.Int(42);
/// let a = Value.Arr([s, n]);
/// a.asArray();  // Some([.Str("hello"), .Int(42)])
/// ```
///
/// # Representation
///
/// Tagged enum. Scalar cases (`.Null`, `.Boolean`, `.Int`, `.Float`)
/// are inline; `.Str`, `.Arr`, and `.Obj` hold heap-backed
/// collections.
public enum Value: Cloneable {
    /// A null/missing value. Maps to JSON `null`, TOML omission, etc.
    case Null

    /// A boolean value (`true` / `false`).
    case Boolean(std.core.Bool)

    /// A 64-bit signed integer.
    case Int(Int64)

    /// A 64-bit IEEE 754 floating-point number.
    case Float(Float64)

    /// A UTF-8 string.
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
    /// Returns a deep copy of this value, recursively cloning any
    /// nested strings, arrays, and objects.
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

    /// Returns `true` if this value is `.Null`.
    ///
    /// # Examples
    ///
    /// ```
    /// Value.Null.isNull();      // true
    /// Value.Int(0).isNull();    // false
    /// ```
    public func isNull() -> Bool {
        match self {
            .Null => true,
            _ => false
        }
    }

    /// Extracts the boolean payload, or `None` if not a `.Boolean`.
    ///
    /// # Examples
    ///
    /// ```
    /// Value.Boolean(true).asBool();   // Some(true)
    /// Value.Int(1).asBool();          // None
    /// ```
    public func asBool() -> Optional[Bool] {
        match self {
            .Boolean(b) => .Some(b),
            _ => .None
        }
    }

    /// Extracts the integer payload, or `None` if not an `.Int`.
    ///
    /// # Examples
    ///
    /// ```
    /// Value.Int(42).asInt();       // Some(42)
    /// Value.Float(1.0).asInt();    // None
    /// ```
    public func asInt() -> Optional[Int64] {
        match self {
            .Int(n) => .Some(n),
            _ => .None
        }
    }

    /// Extracts the float payload, or `None` if not a `.Float`.
    ///
    /// # Examples
    ///
    /// ```
    /// Value.Float(3.14).asFloat();   // Some(3.14)
    /// Value.Int(3).asFloat();        // None
    /// ```
    public func asFloat() -> Optional[Float64] {
        match self {
            .Float(f) => .Some(f),
            _ => .None
        }
    }

    /// Extracts the string payload, or `None` if not a `.Str`.
    ///
    /// # Examples
    ///
    /// ```
    /// Value.Str("hi").asString();    // Some("hi")
    /// Value.Null.asString();         // None
    /// ```
    public func asString() -> Optional[String] {
        match self {
            .Str(s) => .Some(s),
            _ => .None
        }
    }

    /// Extracts the array payload, or `None` if not an `.Arr`.
    ///
    /// # Examples
    ///
    /// ```
    /// Value.Arr([Value.Int(1)]).asArray();   // Some([.Int(1)])
    /// Value.Null.asArray();                  // None
    /// ```
    public func asArray() -> Optional[Array[Value]] {
        match self {
            .Arr(arr) => .Some(arr),
            _ => .None
        }
    }

    /// Extracts the object payload, or `None` if not an `.Obj`.
    ///
    /// # Examples
    ///
    /// ```
    /// let obj = Value.Obj(["k": Value.Int(1)]);
    /// obj.asObject();            // Some({"k": .Int(1)})
    /// Value.Null.asObject();     // None
    /// ```
    public func asObject() -> Optional[Dictionary[String, Value]] {
        match self {
            .Obj(obj) => .Some(obj),
            _ => .None
        }
    }

    /// Looks up a key in this value if it is an `.Obj`.
    ///
    /// Returns `None` both when the key is missing and when this value
    /// is not an object.
    ///
    /// # Examples
    ///
    /// ```
    /// let obj = Value.Obj(["x": Value.Int(1)]);
    /// obj.value(forKey: "x");        // Some(.Int(1))
    /// obj.value(forKey: "missing");  // None
    /// Value.Null.value(forKey: "x"); // None
    /// ```
    public func value(forKey key: String) -> Optional[Value] {
        match self {
            .Obj(obj) => obj(key),
            _ => .None
        }
    }

    /// Returns a short human-readable name for this value's type,
    /// useful in error messages.
    ///
    /// # Examples
    ///
    /// ```
    /// Value.Null.typeName();         // "null"
    /// Value.Str("hi").typeName();    // "string"
    /// Value.Arr([]).typeName();      // "array"
    /// ```
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
