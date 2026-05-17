/// Request body types — text, raw bytes, and URL-encoded form data.

module swoop.body

import http.wire.(stringToBytes)
import http.url.(encodeQueryString)

// ============================================================================
// BODY ENUM
// ============================================================================

/// The body of an HTTP request.
public enum Body: Cloneable {
    /// A plain text body. Does not set Content-Type automatically.
    case Text(String)

    /// A raw binary body. Does not set Content-Type automatically.
    case Bytes(Array[UInt8])

    /// A URL-encoded form body. Sets Content-Type to application/x-www-form-urlencoded.
    case Form(Array[(String, String)])
}

// ============================================================================
// BODY HELPERS
// ============================================================================

extend Body {
    public func clone() -> Body {
        match self {
            .Text(s) => Body.Text(s.clone()),
            .Bytes(b) => Body.Bytes(b.clone()),
            .Form(pairs) => Body.Form(pairs.clone())
        }
    }

    /// Returns the bytes for this body.
    public func toBytes() -> Array[UInt8] {
        match self {
            .Text(s) => stringToBytes(s),
            .Bytes(b) => b,
            .Form(pairs) => stringToBytes(encodeQueryString(pairs))
        }
    }

    /// Returns the byte count of this body.
    public func byteCount() -> Int64 {
        match self {
            .Text(s) => s.byteCount,
            .Bytes(b) => b.count,
            .Form(pairs) => encodeQueryString(pairs).byteCount
        }
    }
}
