// HTTP request methods

module http.method

/// HTTP request methods.
public enum HttpMethod {
    case Get
    case Post
    case Put
    case Delete
    case Patch
    case Head
    case Options

    /// Returns the method name as an uppercase string (e.g. "GET").
    public func toString() -> String {
        match self {
            .Get => "GET",
            .Post => "POST",
            .Put => "PUT",
            .Delete => "DELETE",
            .Patch => "PATCH",
            .Head => "HEAD",
            .Options => "OPTIONS"
        }
    }

    /// Returns true if this method typically carries a request body.
    public func hasBody() -> Bool {
        match self {
            .Post => true,
            .Put => true,
            .Patch => true,
            _ => false
        }
    }
}

/// Parses a string like "GET" into an HttpMethod, or None if unrecognized.
public func parseMethod(s: String) -> HttpMethod? {
    if s.equals("GET") { return .Some(.Get) }
    if s.equals("POST") { return .Some(.Post) }
    if s.equals("PUT") { return .Some(.Put) }
    if s.equals("DELETE") { return .Some(.Delete) }
    if s.equals("PATCH") { return .Some(.Patch) }
    if s.equals("HEAD") { return .Some(.Head) }
    if s.equals("OPTIONS") { return .Some(.Options) }
    .None
}
