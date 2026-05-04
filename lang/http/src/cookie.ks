/// HTTP cookie construction, serialization, and parsing.
///
/// `Cookie` builds `Set-Cookie` header values with common attributes
/// (path, max-age, HttpOnly, Secure, SameSite). The companion
/// `parseCookieHeader` function parses incoming `Cookie` request
/// headers into `(name, value)` pairs.
///
/// # Examples
///
/// ```
/// var c = Cookie(name: "session", value: "abc123");
/// c.httpOnly = true;
/// c.secure = true;
/// c.maxAge = 3600;
/// c.toHeaderValue();
/// // "session=abc123; Path=/; Max-Age=3600; HttpOnly; Secure; SameSite=Lax"
///
/// let pairs = parseCookieHeader("a=1; b=2");
/// // [("a", "1"), ("b", "2")]
/// ```

module http.cookie

/// A `Set-Cookie` value with name, value, and optional attributes.
///
/// Defaults to a session cookie (`maxAge = -1`, meaning no `Max-Age`
/// directive) with `Path=/` and `SameSite=Lax`. Mutate the public
/// fields to customize, then call `toHeaderValue()` to produce the
/// header string.
///
/// # Examples
///
/// ```
/// var c = Cookie(name: "token", value: "xyz");
/// c.path = "/api";
/// c.httpOnly = true;
/// c.toHeaderValue();
/// // "token=xyz; Path=/api; HttpOnly; SameSite=Lax"
/// ```
///
/// # Representation
///
/// Seven fields: `name`, `value`, `path`, `maxAge`, `httpOnly`,
/// `secure`, and `sameSite`. All are public and mutable.
public struct Cookie: Cloneable {
    /// The cookie name.
    public var name: String
    /// The cookie value (not percent-encoded by this type).
    public var value: String
    /// The `Path` attribute. Defaults to `"/"`.
    public var path: String
    /// The `Max-Age` attribute in seconds. A negative value (the
    /// default) means the attribute is omitted, creating a session
    /// cookie.
    public var maxAge: Int64
    /// Whether the `HttpOnly` flag is set.
    public var httpOnly: Bool
    /// Whether the `Secure` flag is set.
    public var secure: Bool
    /// The `SameSite` attribute. Defaults to `"Lax"`. Set to an empty
    /// string to omit the attribute entirely.
    public var sameSite: String

    /// @name Session Cookie
    /// Creates a session cookie with `Path=/`, `SameSite=Lax`, and no
    /// `Max-Age`, `HttpOnly`, or `Secure` directives.
    ///
    /// # Examples
    ///
    /// ```
    /// let c = Cookie(name: "id", value: "42");
    /// c.toHeaderValue();  // "id=42; Path=/; SameSite=Lax"
    /// ```
    public init(name: String, value: String) {
        self.name = name;
        self.value = value;
        self.path = "/";
        self.maxAge = -1;
        self.httpOnly = false;
        self.secure = false;
        self.sameSite = "Lax"
    }

    /// Serializes this cookie into a `Set-Cookie` header value string.
    ///
    /// Attributes are appended only when active: `Max-Age` when
    /// non-negative, `HttpOnly` and `Secure` when `true`, `SameSite`
    /// when non-empty.
    ///
    /// # Examples
    ///
    /// ```
    /// var c = Cookie(name: "s", value: "v");
    /// c.secure = true;
    /// c.maxAge = 600;
    /// c.toHeaderValue();
    /// // "s=v; Path=/; Max-Age=600; Secure; SameSite=Lax"
    /// ```
    public func toHeaderValue() -> String {
        var result = String();
        result.append(self.name);
        result.append("=");
        result.append(self.value);
        result.append("; Path=");
        result.append(self.path);
        if self.maxAge >= 0 {
            result.append("; Max-Age=");
            result.append(self.maxAge.format())
        }
        if self.httpOnly {
            result.append("; HttpOnly")
        }
        if self.secure {
            result.append("; Secure")
        }
        if self.sameSite.byteCount > 0 {
            result.append("; SameSite=");
            result.append(self.sameSite)
        }
        result
    }

    public func clone() -> Cookie {
        Cookie(self.name.clone(), self.value.clone())
    }
}

/// Parses a `Cookie` request header into `(name, value)` pairs.
///
/// Splits on `"; "` (semicolon-space), then on the first `=` in each
/// segment. Whitespace around names and values is trimmed. Segments
/// without `=` are silently skipped.
///
/// # Examples
///
/// ```
/// parseCookieHeader("session=abc; theme=dark");
/// // [("session", "abc"), ("theme", "dark")]
///
/// parseCookieHeader("");  // []
/// ```
public func parseCookieHeader(headerValue: String) -> Array[(String, String)] {
    var result = Array[(String, String)]();
    var parts = headerValue.split("; ");
    while let .Some(part) = parts.next() {
        let trimmedPart = part.trimmed();
        match trimmedPart.find("=") {
            .Some(eqIdx) => {
                let name = trimmedPart.substringBytes(from: 0, to: eqIdx).trimmed();
                let value = trimmedPart.substringBytes(from: eqIdx + 1, to: trimmedPart.byteCount).trimmed();
                result.append((name, value))
            },
            .None => {}
        }
    }
    result
}
