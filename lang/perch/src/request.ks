/// HTTP request model for Perch handlers and middleware.

module perch

import http.method.(HttpMethod)
import http.headers.(Headers)
import http.url.(parseUrl, parseQueryString)
import http.cookie.(parseCookieHeader)

/// A parsed HTTP request with route parameters and middleware state.
///
/// Combines the raw HTTP data (method, path, headers, body) with the path
/// parameters extracted during route matching. Per-request state that
/// middleware produces travels in the typed context, not on the request.
///
/// Query parameters and cookies are parsed eagerly at construction
/// time and stored for O(1) repeated access.
///
/// # Examples
///
/// ```
/// // In a middleware:
/// guard let some header = request.header("Authorization") else {
///     return .Respond(Response.unauthorized())
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
    /// Path parameter values in pattern order — what `Path[T]` extracts.
    public var pathValues: Array[String]
    public var queryParams: Array[(String, String)]
    public var cookies: Array[(String, String)]

    /// Returns the first value of a header by name (case-insensitive).
    public func header(name: String) -> String? = self.headers.value(forName: name)

    /// Returns a path parameter by name, or `None`.
    ///
    /// Path parameters are extracted during route matching from `:name`
    /// segments in the route pattern.
    public func param(name: String) -> String? = self.pathParams(name)

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
        var copy = Request(self.method, url: String(), headers: self.headers.clone(), body: self.body.clone());
        copy.path = self.path.clone();
        copy.segments = self.segments.clone();
        copy.queryString = self.queryString.clone();
        copy.pathParams = self.pathParams.clone();
        copy.pathValues = self.pathValues.clone();
        copy.queryParams = self.queryParams.clone();
        copy.cookies = self.cookies.clone();
        copy
    }
}

extend Request {
    /// A request for `url` (path plus optional `?query`), as if parsed off
    /// the wire. For testing handlers with `App.respond(to:)`.
    public init(method: HttpMethod, url url: String, headers headers: Headers, body body: String) {
        let parsed = parseUrl(url);
        self.cookies = match headers.value(forName: "Cookie") {
            .Some(cookieHeader) => parseCookieHeader(cookieHeader),
            .None => Array[(String, String)]()
        };
        self.method = method;
        self.queryParams = parseQueryString(parsed.queryString);
        self.path = parsed.path;
        self.segments = parsed.segments;
        self.queryString = parsed.queryString;
        self.headers = headers;
        self.body = body;
        self.pathParams = [:];
        self.pathValues = []
    }
}
