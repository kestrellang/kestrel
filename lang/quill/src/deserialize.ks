// Quill Deserialize protocol and built-in conformances

module quill.deserialize

import quill.value.(Value)
import quill.error.(DeserializeError, DeserializeErrorKind)

// ============================================================================
// DESERIALIZE PROTOCOL
// ============================================================================

/// A type that can be constructed from a Value during deserialization.
public protocol Deserialize {
    static func fromValue(value: Value) -> Result[Self, DeserializeError]
}

// ============================================================================
// BUILT-IN CONFORMANCES
// ============================================================================

extend Bool: Deserialize {
    public static func fromValue(value: Value) -> Result[Bool, DeserializeError] {
        match value {
            .Boolean(b) => .Ok(b),
            _ => .Err(DeserializeError.typeMismatch(expected: "bool", got: value.typeName()))
        }
    }
}

extend Int64: Deserialize {
    public static func fromValue(value: Value) -> Result[Int64, DeserializeError] {
        match value {
            .Int(n) => .Ok(n),
            _ => .Err(DeserializeError.typeMismatch(expected: "int", got: value.typeName()))
        }
    }
}

extend Float64: Deserialize {
    public static func fromValue(value: Value) -> Result[Float64, DeserializeError] {
        match value {
            .Float(f) => .Ok(f),
            .Int(n) => .Ok(Float64(from: n)),
            _ => .Err(DeserializeError.typeMismatch(expected: "float", got: value.typeName()))
        }
    }
}

extend String: Deserialize {
    public static func fromValue(value: Value) -> Result[String, DeserializeError] {
        match value {
            .Str(s) => .Ok(s),
            _ => .Err(DeserializeError.typeMismatch(expected: "string", got: value.typeName()))
        }
    }
}

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

extend Value: Deserialize {
    public static func fromValue(value: Value) -> Result[Value, DeserializeError] {
        .Ok(value)
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Looks up a required key in an Object value.
/// Returns the value or an error if the key is missing or value is not an Object.
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

/// Looks up an optional key in an Object value.
/// Returns Some(value) if the key exists, None if missing, or an error if not an Object.
public func findKeyOpt(from value: Value, key: String) -> Result[Optional[Value], DeserializeError] {
    match value {
        .Obj(obj) => .Ok(obj(key)),
        _ => .Err(DeserializeError.typeMismatch(expected: "object", got: value.typeName()))
    }
}

/// Extracts a string from an Object by key.
public func extractString(from value: Value, key: String) -> Result[String, DeserializeError] {
    let v = try findKey(from: value, key);
    match v {
        .Str(s) => .Ok(s),
        _ => .Err(DeserializeError.typeMismatch(expected: "string", got: v.typeName()))
    }
}

/// Extracts an integer from an Object by key.
public func extractInt(from value: Value, key: String) -> Result[Int64, DeserializeError] {
    let v = try findKey(from: value, key);
    match v {
        .Int(n) => .Ok(n),
        _ => .Err(DeserializeError.typeMismatch(expected: "int", got: v.typeName()))
    }
}

/// Extracts a float from an Object by key.
public func extractFloat(from value: Value, key: String) -> Result[Float64, DeserializeError] {
    let v = try findKey(from: value, key);
    match v {
        .Float(f) => .Ok(f),
        .Int(n) => .Ok(Float64(from: n)),
        _ => .Err(DeserializeError.typeMismatch(expected: "float", got: v.typeName()))
    }
}

/// Extracts a boolean from an Object by key.
public func extractBool(from value: Value, key: String) -> Result[Bool, DeserializeError] {
    let v = try findKey(from: value, key);
    match v {
        .Boolean(b) => .Ok(b),
        _ => .Err(DeserializeError.typeMismatch(expected: "bool", got: v.typeName()))
    }
}
