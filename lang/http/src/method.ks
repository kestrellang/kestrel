/// HTTP request method constants and parsing.
///
/// Provides the seven standard methods as an enum with string
/// conversion, body-presence queries, and a parser for incoming
/// request lines.
///
/// # Examples
///
/// ```
/// let m = HttpMethod.Get;
/// m.toString();   // "GET"
/// m.hasBody();    // false
///
/// let p = parseMethod("POST");  // Some(.Post)
/// ```

module http.method

/// The HTTP request methods defined by RFC 9110.
///
/// Each case corresponds to one standard method token. Use
/// `toString()` to get the uppercase wire representation and
/// `hasBody()` to check whether the method conventionally carries a
/// request body.
///
/// # Examples
///
/// ```
/// HttpMethod.Post.toString();  // "POST"
/// HttpMethod.Post.hasBody();   // true
/// HttpMethod.Get.hasBody();    // false
/// ```
///
/// # Representation
///
/// Nullary tagged enum — no payload on any case.
public enum HttpMethod: Equatable {
    /// `GET` — retrieve a resource.
    case Get
    /// `POST` — submit data to a resource.
    case Post
    /// `PUT` — replace a resource entirely.
    case Put
    /// `DELETE` — remove a resource.
    case Delete
    /// `PATCH` — apply a partial update to a resource.
    case Patch
    /// `HEAD` — like `GET` but without a response body.
    case Head
    /// `OPTIONS` — describe the communication options for a resource.
    case Options

    /// Returns `true` if two methods are the same variant.
    public func isEqual(to other: HttpMethod) -> Bool {
        match (self, other) {
            (.Get, .Get) => true,
            (.Post, .Post) => true,
            (.Put, .Put) => true,
            (.Delete, .Delete) => true,
            (.Patch, .Patch) => true,
            (.Head, .Head) => true,
            (.Options, .Options) => true,
            _ => false
        }
    }

    /// Returns the method name as an uppercase string matching the
    /// HTTP/1.1 wire format.
    ///
    /// # Examples
    ///
    /// ```
    /// HttpMethod.Get.toString();      // "GET"
    /// HttpMethod.Delete.toString();   // "DELETE"
    /// ```
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

    /// Returns `true` if this method conventionally carries a request
    /// body. `Post`, `Put`, and `Patch` return `true`; all others
    /// return `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// HttpMethod.Post.hasBody();     // true
    /// HttpMethod.Get.hasBody();      // false
    /// HttpMethod.Patch.hasBody();    // true
    /// ```
    public func hasBody() -> Bool {
        match self {
            .Post => true,
            .Put => true,
            .Patch => true,
            _ => false
        }
    }
}

/// Parses an uppercase method string (e.g. `"GET"`) into an
/// `HttpMethod`, or `None` if the string is not a recognized method.
///
/// # Examples
///
/// ```
/// parseMethod("GET");      // Some(.Get)
/// parseMethod("POST");     // Some(.Post)
/// parseMethod("UNKNOWN");  // None
/// ```
public func parseMethod(s: String) -> HttpMethod? {
    if s == "GET" { return .Some(.Get) }
    if s == "POST" { return .Some(.Post) }
    if s == "PUT" { return .Some(.Put) }
    if s == "DELETE" { return .Some(.Delete) }
    if s == "PATCH" { return .Some(.Patch) }
    if s == "HEAD" { return .Some(.Head) }
    if s == "OPTIONS" { return .Some(.Options) }
    .None
}
