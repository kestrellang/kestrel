// HTTP headers collection
//
// Case-insensitive header lookup backed by an Array of pairs.
// Uses Array rather than Dictionary because HTTP headers are ordered
// and can have multiple values for the same name.

module http.headers

/// A collection of HTTP headers.
///
/// Headers are stored as ordered name-value pairs. Lookups are
/// case-insensitive per the HTTP spec.
public struct Headers: Cloneable {
    var entries: Array[(String, String)]

    /// Creates an empty headers collection.
    public init() {
        self.entries = Array[(String, String)]()
    }

    /// Returns the first value for the given header name (case-insensitive),
    /// or None if not present.
    public func value(forName name: String) -> String? {
        var i: Int64 = 0;
        while i < self.entries.count {
            let pair = self.entries(unchecked: i);
            if pair.0.equalsCaseInsensitive(name) {
                return .Some(pair.1)
            }
            i = i + 1
        }
        .None
    }

    /// Returns all values for the given header name (case-insensitive).
    public func values(forName name: String) -> Array[String] {
        var result = Array[String]();
        var i: Int64 = 0;
        while i < self.entries.count {
            let pair = self.entries(unchecked: i);
            if pair.0.equalsCaseInsensitive(name) {
                result.append(pair.1)
            }
            i = i + 1
        }
        result
    }

    /// Sets a header, removing any existing values for the name.
    public mutating func setValue(name: String, value: String) {
        self.remove(name);
        self.entries.append((name, value))
    }

    /// Adds a header value without removing existing values.
    public mutating func add(name: String, value: String) {
        self.entries.append((name, value))
    }

    /// Removes all values for the given header name (case-insensitive).
    public mutating func remove(name: String) {
        var i: Int64 = 0;
        while i < self.entries.count {
            let pair = self.entries(unchecked: i);
            if pair.0.equalsCaseInsensitive(name) {
                let _ = self.entries.remove(at: i);
            } else {
                i = i + 1
            }
        }
    }

    /// Returns true if the header exists (case-insensitive).
    public func has(name: String) -> Bool {
        self.value(forName: name).isSome()
    }

    /// The number of header entries.
    public var count: Int64 {
        get { self.entries.count }
    }

    public func clone() -> Headers {
        var h = Headers();
        h.entries = self.entries.clone();
        h
    }
}
