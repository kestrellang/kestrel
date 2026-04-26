// Quill serialization error types

module quill.error


// ============================================================================
// SERIALIZE ERROR
// ============================================================================

/// The kind of serialization error that occurred.
public enum SerializeErrorKind: Cloneable {
    /// The value type is not supported for serialization.
    case UnsupportedValue(String)

    /// A custom error message.
    case Custom(String)

    public func clone() -> SerializeErrorKind {
        match self {
            .UnsupportedValue(msg) => .UnsupportedValue(msg.clone()),
            .Custom(msg) => .Custom(msg.clone())
        }
    }
}

/// An error that occurred during serialization.
public struct SerializeError: Cloneable {
    public var kind: SerializeErrorKind

    public init(kind: SerializeErrorKind) {
        self.kind = kind;
    }

    public func clone() -> SerializeError {
        SerializeError(self.kind.clone())
    }

    /// Returns a human-readable description of the error.
    public func description() -> String {
        match self.kind {
            .UnsupportedValue(msg) => {
                var s = String();
                s.append("serialize error: unsupported value: ");
                s.append(msg);
                s
            },
            .Custom(msg) => {
                var s = String();
                s.append("serialize error: ");
                s.append(msg);
                s
            }
        }
    }

    /// Creates an error for an unsupported value type.
    public static func unsupportedValue(message: String) -> SerializeError {
        SerializeError(SerializeErrorKind.UnsupportedValue(message))
    }

    /// Creates a custom serialization error.
    public static func custom(message: String) -> SerializeError {
        SerializeError(SerializeErrorKind.Custom(message))
    }
}

// ============================================================================
// DESERIALIZE ERROR
// ============================================================================

/// The kind of deserialization error that occurred.
public enum DeserializeErrorKind: Cloneable {
    /// Expected one type but got another.
    case TypeMismatch(String, String)

    /// A required key was not found in the object.
    case MissingKey(String)

    /// The value was invalid for the target type.
    case InvalidValue(String)

    /// A custom error message.
    case Custom(String)

    public func clone() -> DeserializeErrorKind {
        match self {
            .TypeMismatch(a, b) => .TypeMismatch(a.clone(), b.clone()),
            .MissingKey(k) => .MissingKey(k.clone()),
            .InvalidValue(v) => .InvalidValue(v.clone()),
            .Custom(msg) => .Custom(msg.clone())
        }
    }
}

/// An error that occurred during deserialization.
public struct DeserializeError: Cloneable {
    public var kind: DeserializeErrorKind
    public var path: Array[String]

    public init(kind: DeserializeErrorKind, path: Array[String]) {
        self.kind = kind;
        self.path = path;
    }

    public func clone() -> DeserializeError {
        DeserializeError(self.kind.clone(), self.path.clone())
    }

    /// Returns a human-readable description of the error.
    public func description() -> String {
        let msg = match self.kind {
            .TypeMismatch(expected, got) => {
                var s = String();
                s.append("type mismatch: expected ");
                s.append(expected);
                s.append(", got ");
                s.append(got);
                s
            },
            .MissingKey(key) => {
                var s = String();
                s.append("missing key: ");
                s.append(key);
                s
            },
            .InvalidValue(msg) => {
                var s = String();
                s.append("invalid value: ");
                s.append(msg);
                s
            },
            .Custom(msg) => msg
        };
        if self.path.isEmpty {
            var s = String();
            s.append("deserialize error: ");
            s.append(msg);
            s
        } else {
            var pathStr = String();
            var i: Int64 = 0;
            while i < self.path.count {
                if i > 0 {
                    pathStr.append(".")
                }
                pathStr.append(self.path(unchecked: i));
                i = i + 1
            }
            var s = String();
            s.append("deserialize error at ");
            s.append(pathStr);
            s.append(": ");
            s.append(msg);
            s
        }
    }

    /// Creates a type mismatch error.
    public static func typeMismatch(expected expected: String, got got: String) -> DeserializeError {
        DeserializeError(DeserializeErrorKind.TypeMismatch(expected, got), Array[String]())
    }

    /// Creates a missing key error.
    public static func missingKey(key: String) -> DeserializeError {
        DeserializeError(DeserializeErrorKind.MissingKey(key), Array[String]())
    }

    /// Creates an invalid value error.
    public static func invalidValue(message: String) -> DeserializeError {
        DeserializeError(DeserializeErrorKind.InvalidValue(message), Array[String]())
    }

    /// Creates a custom deserialization error.
    public static func custom(message: String) -> DeserializeError {
        DeserializeError(DeserializeErrorKind.Custom(message), Array[String]())
    }
}
