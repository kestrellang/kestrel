/// Serialize protocol and built-in conformances for converting user
/// types into `Value` trees.
///
/// Conform your types to `Serialize` so they can be encoded by any
/// `Format` implementation (JSON, TOML, etc.) without coupling to a
/// specific wire format.
///
/// # Examples
///
/// ```
/// let v = try 42.toValue();          // Ok(.Int(42))
/// let s = try "hello".toValue();     // Ok(.Str("hello"))
/// let a = try [1, 2, 3].toValue();   // Ok(.Arr([.Int(1), .Int(2), .Int(3)]))
/// ```

module quill.serialize

import quill.value.(Value)
import quill.error.(SerializeError)

// ============================================================================
// SERIALIZE PROTOCOL
// ============================================================================

/// A type that can be converted to a `Value` for serialization.
///
/// The conversion is fallible — `toValue` returns a `Result` so
/// implementations can report unsupported or invalid states without
/// panicking.
///
/// # Examples
///
/// ```
/// // Built-in conformance:
/// let v = try "hello".toValue();  // Ok(.Str("hello"))
///
/// // Custom type:
/// // extend MyStruct: Serialize {
/// //     public func toValue() -> Value throws SerializeError {
/// //         var obj: [String: Value] = [:];
/// //         obj.insert("field", try self.field.toValue());
/// //         .Ok(Value.Obj(obj))
/// //     }
/// // }
/// ```
public protocol Serialize {
    func toValue() -> Value throws SerializeError
}

// ============================================================================
// BUILT-IN CONFORMANCES
// ============================================================================

/// Serializes to `.Boolean`.
extend Bool: Serialize {
    public func toValue() -> Value throws SerializeError {
        .Ok(Value.Boolean(self))
    }
}

/// Serializes to `.Int`.
extend Int64: Serialize {
    public func toValue() -> Value throws SerializeError {
        .Ok(Value.Int(self))
    }
}

/// Serializes to `.Float`.
extend Float64: Serialize {
    public func toValue() -> Value throws SerializeError {
        .Ok(Value.Float(self))
    }
}

/// Serializes to `.Str`.
extend String: Serialize {
    public func toValue() -> Value throws SerializeError {
        .Ok(Value.Str(self))
    }
}

/// Serializes to the inner value, or `.Null` for `.None`.
extend Optional[T]: Serialize where T: Serialize {
    public func toValue() -> Value throws SerializeError {
        match self {
            .Some(inner) => inner.toValue(),
            .None => .Ok(Value.Null)
        }
    }
}

/// Serializes each element in order, producing `.Arr`.
extend Array[T]: Serialize where T: Serialize {
    public func toValue() -> Value throws SerializeError {
        var result: [Value] = [];
        for item in self {
            result.append(try item.toValue())
        }
        .Ok(Value.Arr(result))
    }
}

/// Identity — a `Value` is already a `Value`.
extend Value: Serialize {
    public func toValue() -> Value throws SerializeError {
        .Ok(self)
    }
}
