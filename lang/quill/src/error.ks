/// Error types for serialization and deserialization failures.
///
/// Two parallel hierarchies — `SerializeError` / `SerializeErrorKind`
/// for encoding, and `DeserializeError` / `DeserializeErrorKind` for
/// decoding. Deserialization errors carry an optional `path` that
/// records the key trail from the document root to the failing node,
/// making nested-object errors easy to diagnose.
///
/// # Examples
///
/// ```
/// let e = DeserializeError.typeMismatch(expected: "int", got: "string");
/// e.description();  // "deserialize error: type mismatch: expected int, got string"
///
/// let e2 = SerializeError.custom(message: "overflow");
/// e2.description();  // "serialize error: overflow"
/// ```

module quill.error


// ============================================================================
// SERIALIZE ERROR
// ============================================================================

/// Discriminant for `SerializeError`, describing what went wrong.
///
/// # Representation
///
/// Tagged enum. Each case carries a single `String` payload with a
/// human-readable message.
public enum SerializeErrorKind: Cloneable {
    /// The value type is not supported for serialization.
    case UnsupportedValue(String)

    /// A free-form error message.
    case Custom(String)

    public func clone() -> SerializeErrorKind {
        match self {
            .UnsupportedValue(msg) => .UnsupportedValue(msg.clone()),
            .Custom(msg) => .Custom(msg.clone())
        }
    }
}

/// An error that occurred while converting a user type to a `Value`.
///
/// Wraps a `SerializeErrorKind` discriminant. Use the static
/// convenience constructors (`unsupportedValue`, `custom`) rather than
/// building the kind by hand.
///
/// # Examples
///
/// ```
/// let e = SerializeError.unsupportedValue(message: "functions");
/// e.description();  // "serialize error: unsupported value: functions"
/// ```
///
/// # Representation
///
/// A single `kind` field holding the `SerializeErrorKind`.
public struct SerializeError: Cloneable {
    /// The discriminated error kind.
    public var kind: SerializeErrorKind

    /// @name From Kind
    /// Wraps a `SerializeErrorKind` in a `SerializeError`.
    public init(kind: SerializeErrorKind) {
        self.kind = kind;
    }

    public func clone() -> SerializeError {
        SerializeError(self.kind.clone())
    }

    /// Returns a human-readable description of the error.
    ///
    /// # Examples
    ///
    /// ```
    /// SerializeError.custom(message: "oops").description();
    /// // "serialize error: oops"
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// SerializeError.unsupportedValue(message: "closure");
    /// ```
    public static func unsupportedValue(message: String) -> SerializeError {
        SerializeError(SerializeErrorKind.UnsupportedValue(message))
    }

    /// Creates a free-form serialization error.
    ///
    /// # Examples
    ///
    /// ```
    /// SerializeError.custom(message: "depth limit exceeded");
    /// ```
    public static func custom(message: String) -> SerializeError {
        SerializeError(SerializeErrorKind.Custom(message))
    }
}

// ============================================================================
// DESERIALIZE ERROR
// ============================================================================

/// Discriminant for `DeserializeError`, describing what went wrong.
///
/// # Representation
///
/// Tagged enum. `.TypeMismatch` carries two strings (expected, got);
/// the others carry one.
public enum DeserializeErrorKind: Cloneable {
    /// Expected one type but got another. The two payloads are the
    /// expected type name and the actual type name.
    case TypeMismatch(String, String)

    /// A required key was not found in the object.
    case MissingKey(String)

    /// The value was syntactically valid but semantically wrong for
    /// the target type (e.g. an out-of-range integer).
    case InvalidValue(String)

    /// A free-form error message.
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

/// An error that occurred while constructing a user type from a
/// `Value`.
///
/// Carries a `kind` discriminant and an optional `path` — an array of
/// key names from the document root to the node where the error was
/// detected. The path starts empty and can be extended by callers as
/// errors propagate upward through nested objects.
///
/// # Examples
///
/// ```
/// let e = DeserializeError.missingKey("name");
/// e.description();
/// // "deserialize error: missing key: name"
/// ```
///
/// # Representation
///
/// Two fields: a `DeserializeErrorKind` and an `Array[String]` path.
public struct DeserializeError: Cloneable {
    /// The discriminated error kind.
    public var kind: DeserializeErrorKind

    /// Key trail from the document root to the error site. Empty when
    /// the error is at the top level.
    public var path: Array[String]

    /// @name From Kind and Path
    /// Wraps a kind and an explicit path in a `DeserializeError`.
    public init(kind: DeserializeErrorKind, path: Array[String]) {
        self.kind = kind;
        self.path = path;
    }

    public func clone() -> DeserializeError {
        DeserializeError(self.kind.clone(), self.path.clone())
    }

    /// Returns a human-readable description including the dotted path
    /// (if any) and the error message.
    ///
    /// # Examples
    ///
    /// ```
    /// let e = DeserializeError.typeMismatch(expected: "int", got: "string");
    /// e.description();
    /// // "deserialize error: type mismatch: expected int, got string"
    /// ```
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

    /// Creates a type-mismatch error with an empty path.
    ///
    /// # Examples
    ///
    /// ```
    /// DeserializeError.typeMismatch(expected: "bool", got: "int");
    /// ```
    public static func typeMismatch(expected expected: String, got got: String) -> DeserializeError {
        DeserializeError(DeserializeErrorKind.TypeMismatch(expected, got), Array[String]())
    }

    /// Creates a missing-key error with an empty path.
    ///
    /// # Examples
    ///
    /// ```
    /// DeserializeError.missingKey("id");
    /// ```
    public static func missingKey(key: String) -> DeserializeError {
        DeserializeError(DeserializeErrorKind.MissingKey(key), Array[String]())
    }

    /// Creates an invalid-value error with an empty path.
    ///
    /// # Examples
    ///
    /// ```
    /// DeserializeError.invalidValue(message: "negative count");
    /// ```
    public static func invalidValue(message: String) -> DeserializeError {
        DeserializeError(DeserializeErrorKind.InvalidValue(message), Array[String]())
    }

    /// Creates a free-form deserialization error with an empty path.
    ///
    /// # Examples
    ///
    /// ```
    /// DeserializeError.custom(message: "unexpected EOF");
    /// ```
    public static func custom(message: String) -> DeserializeError {
        DeserializeError(DeserializeErrorKind.Custom(message), Array[String]())
    }
}
