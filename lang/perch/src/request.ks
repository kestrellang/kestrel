/// HTTP request model for Perch handlers and middleware.

module perch.request

import http.method.(HttpMethod)
import http.headers.(Headers)
import http.url.(parseQueryString)
import http.cookie.(parseCookieHeader)

/// A parsed HTTP request with route parameters and middleware state.
///
/// Combines the raw HTTP data (method, path, headers, body) with
/// server-resolved state: path parameters extracted during route matching
/// and a string key-value store that middleware can populate (e.g. an
/// auth middleware sets `"userId"`).
///
/// Requests are value types. The `withValue` and `withPathParams` methods
/// return new copies — the original is unchanged.
///
/// # Examples
///
/// ```
/// // Inside a handler:
/// func handleUser(request: Request, ctx: Ctx) -> Response {
///     let id = match request.param("id") {
///         .Some(id) => id,
///         .None => return Response.badRequest("Missing id")
///     };
///     Response.ok(text: "User " + id)
/// }
/// ```
public struct Request: Cloneable {
    public var method: HttpMethod
    public var path: String
    public var segments: Array[String]
    public var queryString: String
    public var headers: Headers
    public var body: String
    public var pathParams: Dictionary[String, String]
    public var store: Dictionary[String, String]

    /// Returns the first value of a header by name (case-insensitive).
    ///
    /// # Examples
    ///
    /// ```
    /// match request.header("Authorization") {
    ///     .Some(token) => println(token),
    ///     .None => println("no auth header")
    /// }
    /// ```
    public func header(name: String) -> String? = self.headers.value(forName: name)

    /// Returns a path parameter by name, or `None`.
    ///
    /// Path parameters are extracted during route matching from `:name`
    /// segments in the route pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// // Route pattern: "/users/:id"
    /// // Request path:  "/users/42"
    /// request.param("id")  // => .Some("42")
    /// ```
    public func param(name: String) -> String? = self.pathParams(name)

    /// Returns a value from the middleware store, or `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// let userId = request.getValue(forKey: "userId");
    /// ```
    public func getValue(forKey key: String) -> String? = self.store(key)

    /// Returns a new request with the given value added to the middleware store.
    ///
    /// The original request is unchanged — this creates a copy with the
    /// new key-value pair inserted.
    ///
    /// # Examples
    ///
    /// ```
    /// let enriched = request.withValue(forKey: "userId", "42");
    /// ```
    public func withValue(forKey key: String, value: String) -> Request {
        var newStore = self.store.clone();
        let _ = newStore.insert(key, value);
        Request(
            method: self.method,
            path: self.path,
            segments: self.segments,
            queryString: self.queryString,
            headers: self.headers,
            body: self.body,
            pathParams: self.pathParams,
            store: newStore
        )
    }

    /// Returns a new request with the given path parameters set.
    ///
    /// Called internally by the router after matching a route pattern.
    public func withPathParams(params: Dictionary[String, String]) -> Request {
        Request(
            method: self.method,
            path: self.path,
            segments: self.segments,
            queryString: self.queryString,
            headers: self.headers,
            body: self.body,
            pathParams: params,
            store: self.store
        )
    }

    /// Parses the query string and returns the value for a key, or `None`.
    ///
    /// Re-parses the raw query string on every call. O(n) in the number of
    /// query parameters. If you need multiple values, use `queryParams()`
    /// once and search the result.
    ///
    /// # Examples
    ///
    /// ```
    /// // URL: /search?q=kestrel&page=2
    /// request.query("q")     // => .Some("kestrel")
    /// request.query("page")  // => .Some("2")
    /// request.query("sort")  // => .None
    /// ```
    public func query(name: String) -> String? {
        for (key, value) in parseQueryString(self.queryString) {
            if key == name {
                return .Some(value)
            }
        }
        .None
    }

    /// Returns all parsed query parameters as `(name, value)` pairs.
    ///
    /// Re-parses the raw query string on every call. O(n).
    ///
    /// # Examples
    ///
    /// ```
    /// // URL: /search?q=kestrel&page=2
    /// let params = request.queryParams();
    /// // => [("q", "kestrel"), ("page", "2")]
    /// ```
    public func queryParams() -> Array[(String, String)] = parseQueryString(self.queryString)

    /// Returns all cookies from the `Cookie` request header.
    ///
    /// Re-parses the header on every call. O(n) in the number of cookies.
    ///
    /// # Examples
    ///
    /// ```
    /// let all = request.cookies();
    /// // => [("session", "abc123"), ("theme", "dark")]
    /// ```
    public func cookies() -> Array[(String, String)] {
        match self.header("Cookie") {
            .Some(cookieHeader) => parseCookieHeader(cookieHeader),
            .None => Array[(String, String)]()
        }
    }

    /// Returns a specific cookie value by name, or `None`.
    ///
    /// Re-parses the `Cookie` header on every call. O(n). For multiple
    /// lookups, call `cookies()` once and search the result.
    ///
    /// # Examples
    ///
    /// ```
    /// match request.cookie("session") {
    ///     .Some(token) => println("session: " + token),
    ///     .None => println("no session cookie")
    /// }
    /// ```
    public func cookie(name: String) -> String? {
        for (key, value) in self.cookies() {
            if key == name {
                return .Some(value)
            }
        }
        .None
    }

    /// The `Content-Type` header value, or `None` if not present.
    public var contentType: String? {
        self.header("Content-Type")
    }

    public func clone() -> Request {
        Request(
            method: self.method,
            path: self.path.clone(),
            segments: self.segments.clone(),
            queryString: self.queryString.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            pathParams: self.pathParams.clone(),
            store: self.store.clone()
        )
    }
}
