// HTTP request

module perch.request

import http.method.(HttpMethod)
import http.headers.(Headers)
import http.url.(parseQueryString)
import http.cookie.(parseCookieHeader)

/// A parsed HTTP request with route parameters and middleware state.
///
/// Combines the raw HTTP request data with server-resolved state:
/// - Path parameters from route matching (e.g. "/users/:id" -> param("id"))
/// - Key-value store populated by middleware (e.g. auth sets "userId")
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
    public func header(name: String) -> String? {
        self.headers.value(forName: name)
    }

    /// Returns a path parameter by name, or None.
    public func param(name: String) -> String? {
        self.pathParams(name)
    }

    /// Returns a value from the middleware store, or None.
    public func getValue(key: String) -> String? {
        self.store(key)
    }

    /// Returns a new request with the given value added to the store.
    public func withValue(key: String, value: String) -> Request {
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

    /// Parses the query string and returns the value for a key, or None.
    public func query(name: String) -> String? {
        let params = parseQueryString(self.queryString);
        var i: Int64 = 0;
        while i < params.count {
            let pair = params(unchecked: i);
            if pair.0.equals(name) {
                return .Some(pair.1)
            }
            i = i + 1
        }
        .None
    }

    /// Returns all parsed query parameters.
    public func queryParams() -> Array[(String, String)] {
        parseQueryString(self.queryString)
    }

    /// Returns all cookies from the Cookie header.
    public func cookies() -> Array[(String, String)] {
        match self.header("Cookie") {
            .Some(cookieHeader) => parseCookieHeader(cookieHeader),
            .None => Array[(String, String)]()
        }
    }

    /// Returns a specific cookie value by name, or None.
    public func cookie(name: String) -> String? {
        let all = self.cookies();
        var i: Int64 = 0;
        while i < all.count {
            let pair = all(unchecked: i);
            if pair.0.equals(name) {
                return .Some(pair.1)
            }
            i = i + 1
        }
        .None
    }

    /// Returns the Content-Type header value, or None.
    public func contentType() -> String? {
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
