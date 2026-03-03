// Quill Serialize protocol and built-in conformances

module quill.serialize

import quill.value.(Value)
import quill.error.(SerializeError)

// ============================================================================
// SERIALIZE PROTOCOL
// ============================================================================

/// A type that can be converted to a Value for serialization.
public protocol Serialize {
    func toValue() -> Result[Value, SerializeError]
}

// ============================================================================
// BUILT-IN CONFORMANCES
// ============================================================================

extend Bool: Serialize {
    public func toValue() -> Result[Value, SerializeError] {
        .Ok(Value.Boolean(self))
    }
}

extend Int64: Serialize {
    public func toValue() -> Result[Value, SerializeError] {
        .Ok(Value.Int(self))
    }
}

extend Float64: Serialize {
    public func toValue() -> Result[Value, SerializeError] {
        .Ok(Value.Float(self))
    }
}

extend String: Serialize {
    public func toValue() -> Result[Value, SerializeError] {
        .Ok(Value.Str(self))
    }
}

extend Optional[T]: Serialize where T: Serialize {
    public func toValue() -> Result[Value, SerializeError] {
        match self {
            .Some(inner) => inner.toValue(),
            .None => .Ok(Value.Null)
        }
    }
}

extend Array[T]: Serialize where T: Serialize {
    public func toValue() -> Result[Value, SerializeError] {
        var result = Array[Value]();
        var i: Int64 = 0;
        while i < self.count {
            let item = try self(unchecked: i).toValue();
            result.append(item);
            i = i + 1
        }
        .Ok(Value.Arr(result))
    }
}

extend Value: Serialize {
    public func toValue() -> Result[Value, SerializeError] {
        .Ok(self)
    }
}
