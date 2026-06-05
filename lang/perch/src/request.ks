/// HTTP request model for Perch handlers and middleware.

module perch.request

import http.method.(HttpMethod)
import http.headers.(Headers)

/// A parsed HTTP request with route parameters and middleware state.
///
/// Combines the raw HTTP data (method, path, headers, body) with
/// server-resolved state: path parameters extracted during route matching
/// and a string key-value store that middleware can populate (e.g. an
/// auth middleware sets `"userId"`).
///
/// Query parameters and cookies are parsed eagerly at construction
/// time and stored for O(1) repeated access.
///
/// # Examples
///
/// ```
/// func handleUser(request: Request, ctx: Ctx) -> Response {
///     let id = match request.param("id") {
///         .Some(id) => id,
///         .None => return Response.badRequest(Text("Missing id"))
///     };
///     Response.ok(Text("User " + id))
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
    public var queryParams: Array[(String, String)]
    public var cookies: Array[(String, String)]

    /// Returns the first value of a header by name (case-insensitive).
    public func header(name: String) -> String? = self.headers.value(forName: name)

    /// Returns a path parameter by name, or `None`.
    ///
    /// Path parameters are extracted during route matching from `:name`
    /// segments in the route pattern.
    public func param(name: String) -> String? = self.pathParams(name)

    /// Returns a value from the middleware store, or `None`.
    public func getValue(forKey key: String) -> String? = self.store(key)

    /// Returns a query parameter value by name, or `None`.
    ///
    /// Searches the eagerly-parsed query parameters. O(n) in the
    /// number of query parameters.
    public func query(name: String) -> String? {
        for (key, value) in self.queryParams {
            if key == name {
                return .Some(value)
            }
        }
        .None
    }

    /// Returns a specific cookie value by name, or `None`.
    ///
    /// Searches the eagerly-parsed cookies. O(n) in the number of
    /// cookies.
    public func cookie(name: String) -> String? {
        for (key, value) in self.cookies {
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
            store: self.store.clone(),
            queryParams: self.queryParams.clone(),
            cookies: self.cookies.clone()
        )
    }
}
