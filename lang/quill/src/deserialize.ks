/// Deserialize protocol and built-in conformances for constructing
/// user types from `Value` trees.
///
/// Conform your types to `Deserialize` so they can be decoded from
/// any `Format` implementation (JSON, TOML, etc.) without coupling
/// to a specific wire format. Helper functions (`findKey`,
/// `extractString`, etc.) simplify object field extraction.
///
/// # Examples
///
/// ```
/// let v = Value.Int(42);
/// let n = try Int64.fromValue(v);    // Ok(42)
///
/// let v2 = Value.Str("hello");
/// let s = try String.fromValue(v2);  // Ok("hello")
/// ```

module quill.deserialize

import quill.value.(Value)
import quill.error.(DeserializeError, DeserializeErrorKind)

// ============================================================================
// DESERIALIZE PROTOCOL
// ============================================================================

/// A type that can be constructed from a `Value` during
/// deserialization.
///
/// The conversion is fallible — `fromValue` returns a `Result` so
/// implementations can report type mismatches, missing keys, or
/// invalid data without panicking.
///
/// # Examples
///
/// ```
/// // Built-in conformance:
/// let v = Value.Str("hello");
/// let s = try String.fromValue(v);  // Ok("hello")
///
/// // Custom type:
/// // extend MyStruct: Deserialize {
/// //     public static func fromValue(value: Value) -> Result[MyStruct, DeserializeError] {
/// //         let name = try extractString(from: value, key: "name");
/// //         .Ok(MyStruct(name))
/// //     }
/// // }
/// ```
public protocol Deserialize {
    static func fromValue(value: Value) -> Result[Self, DeserializeError]
}

// ============================================================================
// BUILT-IN CONFORMANCES
// ============================================================================

/// Deserializes from `.Boolean`; rejects all other variants.
extend Bool: Deserialize {
    public static func fromValue(value: Value) -> Result[Bool, DeserializeError] {
        match value {
            .Boolean(b) => .Ok(b),
            _ => .Err(DeserializeError.typeMismatch(expected: "bool", got: value.typeName()))
        }
    }
}

/// Deserializes from `.Int`; rejects all other variants.
extend Int64: Deserialize {
    public static func fromValue(value: Value) -> Result[Int64, DeserializeError] {
        match value {
            .Int(n) => .Ok(n),
            _ => .Err(DeserializeError.typeMismatch(expected: "int", got: value.typeName()))
        }
    }
}

/// Deserializes from `.Float`, or widens `.Int` to `Float64`.
extend Float64: Deserialize {
    public static func fromValue(value: Value) -> Result[Float64, DeserializeError] {
        match value {
            .Float(f) => .Ok(f),
            .Int(n) => .Ok(Float64(from: n)),
            _ => .Err(DeserializeError.typeMismatch(expected: "float", got: value.typeName()))
        }
    }
}

/// Deserializes from `.Str`; rejects all other variants.
extend String: Deserialize {
    public static func fromValue(value: Value) -> Result[String, DeserializeError] {
        match value {
            .Str(s) => .Ok(s),
            _ => .Err(DeserializeError.typeMismatch(expected: "string", got: value.typeName()))
        }
    }
}

/// Deserializes `.Null` as `.None`; all other variants are forwarded
/// to the inner type's `fromValue` and wrapped in `.Some`.
extend Optional[T]: Deserialize where T: Deserialize {
    public static func fromValue(value: Value) -> Result[Optional[T], DeserializeError] {
        match value {
            .Null => .Ok(.None),
            _ => {
                let inner = try T.fromValue(value);
                .Ok(.Some(inner))
            }
        }
    }
}

/// Deserializes from `.Arr`, decoding each element via `T.fromValue`.
extend Array[T]: Deserialize where T: Deserialize {
    public static func fromValue(value: Value) -> Result[Array[T], DeserializeError] {
        match value {
            .Arr(arr) => {
                var result = Array[T]();
                var i: Int64 = 0;
                while i < arr.count {
                    let item = try T.fromValue(arr(unchecked: i));
                    result.append(item);
                    i = i + 1
                }
                .Ok(result)
            },
            _ => .Err(DeserializeError.typeMismatch(expected: "array", got: value.typeName()))
        }
    }
}

/// Identity — a `Value` always deserializes to itself.
extend Value: Deserialize {
    public static func fromValue(value: Value) -> Result[Value, DeserializeError] {
        .Ok(value)
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Looks up a required key in an `.Obj` value.
///
/// Returns the value at `key`, or an error if the key is missing or
/// the value is not an object.
///
/// # Examples
///
/// ```
/// let obj = Value.Obj(["name": Value.Str("Alice")]);
/// let v = try findKey(from: obj, key: "name");  // Ok(.Str("Alice"))
/// ```
public func findKey(from value: Value, key: String) -> Result[Value, DeserializeError] {
    match value {
        .Obj(obj) => {
            match obj(key) {
                .Some(v) => .Ok(v),
                .None => .Err(DeserializeError.missingKey(key))
            }
        },
        _ => .Err(DeserializeError.typeMismatch(expected: "object", got: value.typeName()))
    }
}

/// Looks up an optional key in an `.Obj` value.
///
/// Returns `Some(value)` if the key exists, `None` if missing, or an
/// error if the value is not an object.
///
/// # Examples
///
/// ```
/// let obj = Value.Obj(["a": Value.Int(1)]);
/// let v = try findKeyOpt(from: obj, key: "a");  // Ok(Some(.Int(1)))
/// let m = try findKeyOpt(from: obj, key: "b");  // Ok(None)
/// ```
public func findKeyOpt(from value: Value, key: String) -> Result[Optional[Value], DeserializeError] {
    match value {
        .Obj(obj) => .Ok(obj(key)),
        _ => .Err(DeserializeError.typeMismatch(expected: "object", got: value.typeName()))
    }
}

/// Extracts a `String` from an `.Obj` by key.
///
/// Combines `findKey` with a `.Str` type check in one step.
///
/// # Examples
///
/// ```
/// let obj = Value.Obj(["name": Value.Str("Alice")]);
/// let s = try extractString(from: obj, key: "name");  // Ok("Alice")
/// ```
public func extractString(from value: Value, key: String) -> Result[String, DeserializeError] {
    let v = try findKey(from: value, key);
    match v {
        .Str(s) => .Ok(s),
        _ => .Err(DeserializeError.typeMismatch(expected: "string", got: v.typeName()))
    }
}

/// Extracts an `Int64` from an `.Obj` by key.
///
/// Combines `findKey` with an `.Int` type check in one step.
///
/// # Examples
///
/// ```
/// let obj = Value.Obj(["age": Value.Int(30)]);
/// let n = try extractInt(from: obj, key: "age");  // Ok(30)
/// ```
public func extractInt(from value: Value, key: String) -> Result[Int64, DeserializeError] {
    let v = try findKey(from: value, key);
    match v {
        .Int(n) => .Ok(n),
        _ => .Err(DeserializeError.typeMismatch(expected: "int", got: v.typeName()))
    }
}

/// Extracts a `Float64` from an `.Obj` by key.
///
/// Combines `findKey` with a `.Float` type check. Also accepts `.Int`
/// values, widening them to `Float64`.
///
/// # Examples
///
/// ```
/// let obj = Value.Obj(["pi": Value.Float(3.14)]);
/// let f = try extractFloat(from: obj, key: "pi");  // Ok(3.14)
/// ```
public func extractFloat(from value: Value, key: String) -> Result[Float64, DeserializeError] {
    let v = try findKey(from: value, key);
    match v {
        .Float(f) => .Ok(f),
        .Int(n) => .Ok(Float64(from: n)),
        _ => .Err(DeserializeError.typeMismatch(expected: "float", got: v.typeName()))
    }
}

/// Extracts a `Bool` from an `.Obj` by key.
///
/// Combines `findKey` with a `.Boolean` type check in one step.
///
/// # Examples
///
/// ```
/// let obj = Value.Obj(["active": Value.Boolean(true)]);
/// let b = try extractBool(from: obj, key: "active");  // Ok(true)
/// ```
public func extractBool(from value: Value, key: String) -> Result[Bool, DeserializeError] {
    let v = try findKey(from: value, key);
    match v {
        .Boolean(b) => .Ok(b),
        _ => .Err(DeserializeError.typeMismatch(expected: "bool", got: v.typeName()))
    }
}
