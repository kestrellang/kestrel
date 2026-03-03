// HTTP cookie parsing and emission

module http.cookie

/// Represents a Set-Cookie header value.
public struct Cookie: Cloneable {
    public var name: String
    public var value: String
    public var path: String
    public var maxAge: Int64
    public var httpOnly: Bool
    public var secure: Bool
    public var sameSite: String

    /// Creates a session cookie with default settings.
    public init(name: String, value: String) {
        self.name = name;
        self.value = value;
        self.path = "/";
        self.maxAge = -1;
        self.httpOnly = false;
        self.secure = false;
        self.sameSite = "Lax"
    }

    /// Serializes this cookie into a Set-Cookie header value.
    public func toHeaderValue() -> String {
        var result = self.name + "=" + self.value;
        result = result + "; Path=" + self.path;
        if self.maxAge >= 0 {
            result = result + "; Max-Age=" + self.maxAge.format()
        }
        if self.httpOnly {
            result = result + "; HttpOnly"
        }
        if self.secure {
            result = result + "; Secure"
        }
        if self.sameSite.byteCount > 0 {
            result = result + "; SameSite=" + self.sameSite
        }
        result
    }

    public func clone() -> Cookie {
        Cookie(self.name.clone(), self.value.clone())
    }
}

/// Parses a Cookie header value like "name1=val1; name2=val2" into pairs.
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
