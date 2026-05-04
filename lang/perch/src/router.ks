/// HTTP router with path-parameter extraction and route groups.

module perch.router

import http.method.(HttpMethod)
import perch.request.(Request)
import perch.context.(MiddlewareResult)
import perch.response.(Response)

// ============================================================================
// ROUTE
// ============================================================================

/// A route maps an HTTP method + path pattern to middleware and a handler.
///
/// Path patterns support `:name` segments for parameter extraction.
/// For example, `"/users/:id"` matches `"/users/42"` and binds
/// `id` → `"42"` in the request's path params.
public struct Route[T]: Cloneable {
    var method: HttpMethod
    var pattern: String
    var patternSegments: Array[String]
    var middleware: Array[(Request, T) -> MiddlewareResult]
    var handler: (Request, T) -> Response

    public func clone() -> Route[T] {
        Route[T](
            method: self.method,
            pattern: self.pattern.clone(),
            patternSegments: self.patternSegments.clone(),
            middleware: self.middleware.clone(),
            handler: self.handler
        )
    }
}

// ============================================================================
// ROUTE GROUP
// ============================================================================

/// A group of routes sharing a path prefix and middleware.
public struct RouteGroup[T]: Cloneable {
    var prefix: String
    var prefixSegments: Array[String]
    var middleware: Array[(Request, T) -> MiddlewareResult]
    var routes: Array[Route[T]]

    public func clone() -> RouteGroup[T] {
        RouteGroup[T](
            prefix: self.prefix.clone(),
            prefixSegments: self.prefixSegments.clone(),
            middleware: self.middleware.clone(),
            routes: self.routes.clone()
        )
    }
}

// ============================================================================
// GROUP BUILDER
// ============================================================================

/// Builder for creating a route group with shared prefix and middleware.
///
/// # Examples
///
/// ```
/// var api = GroupBuilder[Ctx]("/api");
/// api.use(requireAuth[Ctx]());
/// api.onGet("/users", listUsers);
/// api.onGet("/users/:id", getUser);
/// app.addGroup(api);
/// ```
public struct GroupBuilder[T]: Cloneable {
    var prefix: String
    var middleware: Array[(Request, T) -> MiddlewareResult]
    var routes: Array[Route[T]]

    /// Creates a new group builder with the given path prefix.
    public init(prefix: String) {
        self.prefix = prefix;
        self.middleware = Array[(Request, T) -> MiddlewareResult]();
        self.routes = Array[Route[T]]()
    }

    /// Adds middleware to this group.
    public mutating func use(middleware: (Request, T) -> MiddlewareResult) {
        self.middleware.append(middleware)
    }

    /// Registers a GET route in this group.
    public mutating func onGet(path: String, handler: (Request, T) -> Response) {
        let fullPath = self.prefix + path;
        let segments = splitPathSegments(fullPath);
        self.routes.append(Route[T](
            method: HttpMethod.Get,
            pattern: fullPath,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers a POST route in this group.
    public mutating func onPost(path: String, handler: (Request, T) -> Response) {
        let fullPath = self.prefix + path;
        let segments = splitPathSegments(fullPath);
        self.routes.append(Route[T](
            method: HttpMethod.Post,
            pattern: fullPath,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers a PUT route in this group.
    public mutating func onPut(path: String, handler: (Request, T) -> Response) {
        let fullPath = self.prefix + path;
        let segments = splitPathSegments(fullPath);
        self.routes.append(Route[T](
            method: HttpMethod.Put,
            pattern: fullPath,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers a DELETE route in this group.
    public mutating func onDelete(path: String, handler: (Request, T) -> Response) {
        let fullPath = self.prefix + path;
        let segments = splitPathSegments(fullPath);
        self.routes.append(Route[T](
            method: HttpMethod.Delete,
            pattern: fullPath,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers a PATCH route in this group.
    public mutating func onPatch(path: String, handler: (Request, T) -> Response) {
        let fullPath = self.prefix + path;
        let segments = splitPathSegments(fullPath);
        self.routes.append(Route[T](
            method: HttpMethod.Patch,
            pattern: fullPath,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers a HEAD route in this group.
    public mutating func onHead(path: String, handler: (Request, T) -> Response) {
        let fullPath = self.prefix + path;
        let segments = splitPathSegments(fullPath);
        self.routes.append(Route[T](
            method: HttpMethod.Head,
            pattern: fullPath,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers an OPTIONS route in this group.
    public mutating func onOptions(path: String, handler: (Request, T) -> Response) {
        let fullPath = self.prefix + path;
        let segments = splitPathSegments(fullPath);
        self.routes.append(Route[T](
            method: HttpMethod.Options,
            pattern: fullPath,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    public func clone() -> GroupBuilder[T] {
        var gb = GroupBuilder[T](self.prefix.clone());
        gb.middleware = self.middleware.clone();
        gb.routes = self.routes.clone();
        gb
    }
}

// ============================================================================
// MATCH RESULT
// ============================================================================

/// The result of matching a request against a route.
struct MatchResult[T]: Cloneable {
    var params: Dictionary[String, String]
    var groupMiddleware: Array[(Request, T) -> MiddlewareResult]
    var routeMiddleware: Array[(Request, T) -> MiddlewareResult]
    var handler: (Request, T) -> Response

    func clone() -> MatchResult[T] {
        MatchResult[T](
            params: self.params.clone(),
            groupMiddleware: self.groupMiddleware.clone(),
            routeMiddleware: self.routeMiddleware.clone(),
            handler: self.handler
        )
    }
}

// ============================================================================
// ROUTER
// ============================================================================

/// The main router holding global middleware, route groups, and standalone routes.
///
/// Route matching checks groups first (in registration order), then
/// standalone routes. The first match wins — later routes for the same
/// method and pattern are shadowed.
public struct Router[T]: Cloneable {
    public var globalMiddleware: Array[(Request, T) -> MiddlewareResult]
    var groups: Array[RouteGroup[T]]
    var routes: Array[Route[T]]

    /// Creates an empty router.
    public init() {
        self.globalMiddleware = Array[(Request, T) -> MiddlewareResult]();
        self.groups = Array[RouteGroup[T]]();
        self.routes = Array[Route[T]]()
    }

    /// Adds global middleware that runs on every request.
    public mutating func use(middleware: (Request, T) -> MiddlewareResult) {
        self.globalMiddleware.append(middleware)
    }

    /// Registers a GET route.
    public mutating func onGet(path: String, handler: (Request, T) -> Response) {
        let segments = splitPathSegments(path);
        self.routes.append(Route[T](
            method: HttpMethod.Get,
            pattern: path,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers a POST route.
    public mutating func onPost(path: String, handler: (Request, T) -> Response) {
        let segments = splitPathSegments(path);
        self.routes.append(Route[T](
            method: HttpMethod.Post,
            pattern: path,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers a PUT route.
    public mutating func onPut(path: String, handler: (Request, T) -> Response) {
        let segments = splitPathSegments(path);
        self.routes.append(Route[T](
            method: HttpMethod.Put,
            pattern: path,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers a DELETE route.
    public mutating func onDelete(path: String, handler: (Request, T) -> Response) {
        let segments = splitPathSegments(path);
        self.routes.append(Route[T](
            method: HttpMethod.Delete,
            pattern: path,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers a PATCH route.
    public mutating func onPatch(path: String, handler: (Request, T) -> Response) {
        let segments = splitPathSegments(path);
        self.routes.append(Route[T](
            method: HttpMethod.Patch,
            pattern: path,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers a HEAD route.
    public mutating func onHead(path: String, handler: (Request, T) -> Response) {
        let segments = splitPathSegments(path);
        self.routes.append(Route[T](
            method: HttpMethod.Head,
            pattern: path,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Registers an OPTIONS route.
    public mutating func onOptions(path: String, handler: (Request, T) -> Response) {
        let segments = splitPathSegments(path);
        self.routes.append(Route[T](
            method: HttpMethod.Options,
            pattern: path,
            patternSegments: segments,
            middleware: Array[(Request, T) -> MiddlewareResult](),
            handler: handler
        ))
    }

    /// Adds a route group.
    public mutating func addGroup(group: GroupBuilder[T]) {
        let prefixSegments = splitPathSegments(group.prefix);
        self.groups.append(RouteGroup[T](
            prefix: group.prefix,
            prefixSegments: prefixSegments,
            middleware: group.middleware,
            routes: group.routes
        ))
    }

    /// Finds a matching route for the given method and path segments.
    func findRoute(method: HttpMethod, segments: Array[String]) -> MatchResult[T]? {
        // Check group routes first
        for group in self.groups {
            for route in group.routes {
                if methodMatches(route.method, method) {
                    match matchPath(segments, route.patternSegments) {
                        .Some(params) => {
                            return .Some(MatchResult[T](
                                params: params,
                                groupMiddleware: group.middleware,
                                routeMiddleware: route.middleware,
                                handler: route.handler
                            ))
                        },
                        .None => {}
                    }
                }
            }
        }

        // Check standalone routes
        for route in self.routes {
            if methodMatches(route.method, method) {
                match matchPath(segments, route.patternSegments) {
                    .Some(params) => {
                        return .Some(MatchResult[T](
                            params: params,
                            groupMiddleware: Array[(Request, T) -> MiddlewareResult](),
                            routeMiddleware: route.middleware,
                            handler: route.handler
                        ))
                    },
                    .None => {}
                }
            }
        }

        .None
    }

    public func clone() -> Router[T] {
        var r = Router[T]();
        r.globalMiddleware = self.globalMiddleware.clone();
        r.groups = self.groups.clone();
        r.routes = self.routes.clone();
        r
    }
}

// ============================================================================
// PATH MATCHING
// ============================================================================

/// Matches request path segments against a route pattern.
///
/// Returns a dictionary of extracted path parameters if the segments
/// match, or `None` otherwise. A `:name` segment matches any value
/// and binds it. Literal segments must match exactly.
func matchPath(requestSegments: Array[String], patternSegments: Array[String]) -> Dictionary[String, String]? {
    if requestSegments.count != patternSegments.count {
        return .None
    }

    var params = Dictionary[String, String]();
    for i in 0..<patternSegments.count {
        let pattern = patternSegments(unchecked: i);
        let actual = requestSegments(unchecked: i);

        if pattern.starts(with: ":") {
            let paramName = pattern.substringBytes(from: 1, to: pattern.byteCount);
            let _ = params.insert(paramName, actual);
        } else if not pattern.equals(actual) {
            return .None
        }
    }

    .Some(params)
}

/// Returns true if two HTTP methods are the same.
func methodMatches(expected: HttpMethod, actual: HttpMethod) -> Bool {
    expected.toString().equals(actual.toString())
}

/// Splits a path into non-empty segments.
func splitPathSegments(path: String) -> Array[String] {
    var segments = Array[String]();
    var parts = path.split("/");
    while let .Some(part) = parts.next() {
        if part.byteCount > 0 {
            segments.append(part)
        }
    }
    segments
}
