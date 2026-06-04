/// JSON content type for Swoop HTTP requests.

module swoop.content

import http.wire.(stringToBytes)
import http.content.(Content)
import quill.value.(Value)
import quill.serialize.(Serialize)
import quill.error.(SerializeError)
import quill.json.emitter.(emitJson)

// ============================================================================
// JSON BODY
// ============================================================================

/// JSON content. Sets Content-Type to application/json.
public struct JsonBody: Content, Cloneable {
    private var raw: String

    init(fromString raw: String) {
        self.raw = raw;
    }

    /// Creates a JsonBody from any Serialize-conforming value.
    public init[T](value: T) throws SerializeError where T: Serialize {
        self.raw = match value.toValue() {
            .Ok(v) => emitJson(v),
            .Err(e) => return .Err(e)
        };
    }

    /// Creates a JsonBody from a raw quill Value.
    public init(fromRaw value: Value) {
        self.raw = emitJson(value);
    }

    public func toBytes() -> Array[UInt8] = stringToBytes(self.raw)
    public func byteCount() -> Int64 = self.raw.byteCount
    public func contentType() -> String? = .Some("application/json")

    public func clone() -> JsonBody = JsonBody(fromString: self.raw.clone())
}
