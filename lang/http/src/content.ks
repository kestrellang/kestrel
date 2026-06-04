/// Request content types for HTTP requests.
///
/// The `Content` protocol defines how request data is serialized to bytes
/// and what MIME type it carries. Concrete types — `Text`, `Bytes`, `Form`
/// — each handle one content format.

module http.content

import http.wire.(stringToBytes)
import http.url.(encodeQueryString)

// ============================================================================
// CONTENT PROTOCOL
// ============================================================================

/// A serializable HTTP request body.
public protocol Content {
    /// Serializes the content to bytes for transmission.
    func toBytes() -> Array[UInt8]

    /// Returns the byte count of the serialized content.
    func byteCount() -> Int64

    /// Returns the MIME type for the Content-Type header, or None
    /// to leave it unset.
    func contentType() -> String?
}

// ============================================================================
// TEXT
// ============================================================================

/// Plain text content. Sets Content-Type to text/plain.
public struct Text: Content, Cloneable {
    var value: String

    public init(value: String) {
        self.value = value;
    }

    public func toBytes() -> Array[UInt8] = stringToBytes(self.value)
    public func byteCount() -> Int64 = self.value.byteCount
    public func contentType() -> String? = .Some("text/plain; charset=utf-8")

    public func clone() -> Text = Text(self.value.clone())
}

// ============================================================================
// HTML
// ============================================================================

/// HTML content. Sets Content-Type to text/html.
public struct Html: Content, Cloneable {
    var value: String

    public init(value: String) {
        self.value = value;
    }

    public func toBytes() -> Array[UInt8] = stringToBytes(self.value)
    public func byteCount() -> Int64 = self.value.byteCount
    public func contentType() -> String? = .Some("text/html; charset=utf-8")

    public func clone() -> Html = Html(self.value.clone())
}

// ============================================================================
// BYTES
// ============================================================================

/// Raw binary content. Does not set Content-Type automatically.
public struct Bytes: Content, Cloneable {
    var value: Array[UInt8]

    public init(value: Array[UInt8]) {
        self.value = value;
    }

    public func toBytes() -> Array[UInt8] = self.value
    public func byteCount() -> Int64 = self.value.count
    public func contentType() -> String? = .None

    public func clone() -> Bytes = Bytes(self.value.clone())
}

// ============================================================================
// FORM
// ============================================================================

/// URL-encoded form content. Sets Content-Type to
/// application/x-www-form-urlencoded.
public struct Form: Content, Cloneable {
    var pairs: Array[(String, String)]

    public init(pairs: Array[(String, String)]) {
        self.pairs = pairs;
    }

    public func toBytes() -> Array[UInt8] = stringToBytes(encodeQueryString(self.pairs))
    public func byteCount() -> Int64 = encodeQueryString(self.pairs).byteCount
    public func contentType() -> String? = .Some("application/x-www-form-urlencoded")

    public func clone() -> Form = Form(self.pairs.clone())
}
