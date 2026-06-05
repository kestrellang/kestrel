/// Case-insensitive HTTP header collection.
///
/// Backed by an `Array` of `(name, value)` pairs rather than a
/// dictionary, because HTTP headers are ordered and the same name can
/// appear more than once (e.g. `Set-Cookie`). All name lookups are
/// case-insensitive per RFC 9110.
///
/// # Examples
///
/// ```
/// var h = Headers();
/// h.setValue(name: "Content-Type", value: "text/html");
/// h.add(name: "Set-Cookie", value: "a=1");
/// h.add(name: "Set-Cookie", value: "b=2");
///
/// h.value(forName: "content-type");       // Some("text/html")
/// h.values(forName: "set-cookie").count;  // 2
/// ```

module http.headers

/// A collection of HTTP headers stored as ordered name-value pairs.
///
/// Duplicate names are allowed — use `add` to append and `values` to
/// retrieve all entries for a name. `setValue` replaces all existing
/// entries for the name with a single new one.
///
/// # Examples
///
/// ```
/// var h = Headers();
/// h.add(name: "Accept", value: "text/html");
/// h.add(name: "Accept", value: "application/json");
/// h.values(forName: "Accept").count;  // 2
/// h.setValue(name: "Accept", value: "*/*");
/// h.values(forName: "Accept").count;  // 1
/// ```
///
/// # Representation
///
/// A single `Array[(String, String)]` holding `(name, value)` pairs
/// in insertion order.
public struct Headers: Cloneable {
    /// The backing array of `(name, value)` pairs.
    var entries: Array[(String, String)]

    /// @name Empty
    /// Creates an empty header collection.
    ///
    /// # Examples
    ///
    /// ```
    /// let h = Headers();
    /// h.count;  // 0
    /// ```
    public init() {
        self.entries = Array[(String, String)]()
    }

    /// Returns the first value for the given header name
    /// (case-insensitive), or `None` if not present.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Headers();
    /// h.setValue(name: "Host", value: "example.com");
    /// h.value(forName: "host");     // Some("example.com")
    /// h.value(forName: "missing");  // None
    /// ```
    public func value(forName name: String) -> String? {
        for (key, value) in self.entries {
            if key.equalsIgnoreAsciiCase(name) {
                return .Some(value)
            }
        }
        .None
    }

    /// Returns all values for the given header name
    /// (case-insensitive). Returns an empty array if the name is not
    /// present.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Headers();
    /// h.add(name: "X-Tag", value: "a");
    /// h.add(name: "X-Tag", value: "b");
    /// h.values(forName: "x-tag");  // ["a", "b"]
    /// ```
    public func values(forName name: String) -> Array[String] {
        var result = Array[String]();
        for (key, value) in self.entries {
            if key.equalsIgnoreAsciiCase(name) {
                result.append(value)
            }
        }
        result
    }

    /// Sets a header to a single value, removing any existing entries
    /// with the same name.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Headers();
    /// h.add(name: "X", value: "old");
    /// h.setValue(name: "X", value: "new");
    /// h.values(forName: "X");  // ["new"]
    /// ```
    public mutating func setValue(name: String, value: String) {
        self.remove(name);
        self.entries.append((name, value))
    }

    /// Appends a header entry without removing existing values for the
    /// same name. Use this for headers that allow multiple values
    /// (e.g. `Set-Cookie`).
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Headers();
    /// h.add(name: "Set-Cookie", value: "a=1");
    /// h.add(name: "Set-Cookie", value: "b=2");
    /// h.values(forName: "Set-Cookie").count;  // 2
    /// ```
    public mutating func add(name: String, value: String) {
        self.entries.append((name, value))
    }

    /// Removes all entries for the given header name
    /// (case-insensitive). No-op if the name is not present.
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Headers();
    /// h.add(name: "X", value: "1");
    /// h.remove("X");
    /// h.has(name: "X");  // false
    /// ```
    public mutating func remove(name: String) {
        var i: Int64 = 0;
        while i < self.entries.count {
            let pair = self.entries(unchecked: i);
            if pair.0.equalsIgnoreAsciiCase(name) {
                 self.entries.remove(at: i);
            } else {
                i = i + 1
            }
        }
    }

    /// Returns `true` if at least one entry exists for the given name
    /// (case-insensitive).
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Headers();
    /// h.setValue(name: "Host", value: "example.com");
    /// h.has(name: "Host");     // true
    /// h.has(name: "missing");  // false
    /// ```
    public func has(name: String) -> Bool {
        self.value(forName: name).isSome()
    }

    /// The total number of header entries (including duplicate names).
    public var count: Int64 {
        get { self.entries.count }
    }

    public func clone() -> Headers {
        var h = Headers();
        h.entries = self.entries.clone();
        h
    }
}

// ============================================================================
// WIRE FORMAT
// ============================================================================

extend Headers {
    /// Parses a block of HTTP header lines into a `Headers` collection.
    ///
    /// Expects lines separated by `\r\n`, each in `Name: Value` format.
    /// The caller should strip the request or status line before passing
    /// the remaining header block. Parsing stops at an empty line or the
    /// end of the string.
    ///
    /// # Examples
    ///
    /// ```
    /// let h = Headers.parse(from: "Host: example.com\r\nAccept: */*\r\n");
    /// h.value(forName: "Host");  // Some("example.com")
    /// ```
    public static func parse(from headerBlock: String) -> Headers {
        var headers = Headers();
        for line in headerBlock.split("\r\n") {
            if line.byteCount == 0 {
                break
            }
            match line.firstIndex(of: ":") {
                .Some(colonIdx) => {
                    let name = line.subslice(from: line.start, to: colonIdx.value).trimmed().toOwned();
                    let value = line.subslice(from: colonIdx.value + 1, to: line.end).trimmed().toOwned();
                    headers.add(name, value)
                },
                .None => {}
            }
        }
        headers
    }

    /// Serializes the headers to HTTP/1.1 wire format.
    ///
    /// Returns a string of `Name: Value\r\n` lines, one per entry.
    /// Does not include the trailing blank line — the caller appends
    /// that after adding any protocol headers (Content-Length, etc.).
    ///
    /// # Examples
    ///
    /// ```
    /// var h = Headers();
    /// h.setValue("Content-Type", "text/html");
    /// h.toWireFormat();  // "Content-Type: text/html\r\n"
    /// ```
    public func toWireFormat() -> String {
        var result = String();
        for (name, value) in self.entries {
            result.append(name);
            result.append(": ");
            result.append(value);
            result.append("\r\n");
        }
        result
    }
}
