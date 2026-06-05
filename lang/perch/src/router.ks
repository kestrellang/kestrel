/// HTTP router with path-parameter extraction and route groups.

module perch.router

import http.method.(HttpMethod)
import perch.request.(Request)
import perch.context.(MiddlewareResult, Middleware)
import perch.response.(Response)

// ============================================================================
// ROUTES PROTOCOL
// ============================================================================

/// A type that can register HTTP route handlers.
///
/// Conform to this protocol by implementing `addRoute`. The seven
/// verb methods — `route(get:)`, `route(post:)`, etc. — are provided
/// automatically by the blanket extension.
///
/// # Examples
///
/// ```
/// app.route(get: "/users", listUsers);
/// app.route(post: "/users", createUser);
/// app.route(delete: "/users/:id", deleteUser);
/// ```
public protocol Routes[T] {
    mutating func addRoute(method: HttpMethod, path: String, handler: (Request, T) -> Response)
}

extend Routes {
    /// Registers a GET route.
    public mutating func route(get path: String, handler: (Request, T) -> Response) {
        self.addRoute(HttpMethod.Get, path, handler)
    }

    /// Registers a POST route.
    public mutating func route(post path: String, handler: (Request, T) -> Response) {
        self.addRoute(HttpMethod.Post, path, handler)
    }

    /// Registers a PUT route.
    public mutating func route(put path: String, handler: (Request, T) -> Response) {
        self.addRoute(HttpMethod.Put, path, handler)
    }

    /// Registers a DELETE route.
    public mutating func route(delete path: String, handler: (Request, T) -> Response) {
        self.addRoute(HttpMethod.Delete, path, handler)
    }

    /// Registers a PATCH route.
    public mutating func route(patch path: String, handler: (Request, T) -> Response) {
        self.addRoute(HttpMethod.Patch, path, handler)
    }

    /// Registers a HEAD route.
    public mutating func route(head path: String, handler: (Request, T) -> Response) {
        self.addRoute(HttpMethod.Head, path, handler)
    }

    /// Registers an OPTIONS route.
    public mutating func route(options path: String, handler: (Request, T) -> Response) {
        self.addRoute(HttpMethod.Options, path, handler)
    }
}

// ============================================================================
// ROUTE
// ============================================================================

/// A route maps an HTTP method + path pattern to a handler.
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
/// api.use(RequireAuth[Ctx]());
/// api.route(get: "/users", listUsers);
/// api.route(get: "/users/:id", getUser);
/// app.addGroup(api);
/// ```
public struct GroupBuilder[T]: Cloneable, Routes[T] {
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
    public mutating func use[M](middleware: M) where M: Middleware[T] {
        self.middleware.append({ (req: Request, ctx: T) in middleware.handle(req, ctx) })
    }

    public mutating func addRoute(method: HttpMethod, path: String, handler: (Request, T) -> Response) {
        let fullPath = self.prefix + path;
        let segments = splitPathSegments(fullPath);
        self.routes.append(Route[T](
            method: method,
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
public struct Router[T]: Cloneable, Routes[T] {
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
    public mutating func use[M](middleware: M) where M: Middleware[T] {
        self.globalMiddleware.append({ (req: Request, ctx: T) in middleware.handle(req, ctx) })
    }

    public mutating func addRoute(method: HttpMethod, path: String, handler: (Request, T) -> Response) {
        let segments = splitPathSegments(path);
        self.routes.append(Route[T](
            method: method,
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
        for group in self.groups {
            for route in group.routes {
                if route.method.toString() == method.toString() {
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

        for route in self.routes {
            if route.method.toString() == method.toString() {
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
            let paramName = pattern.asSlice().subslice(from: 1, to: pattern.byteCount).toOwned();
            let _ = params.insert(paramName, actual);
        } else if pattern != actual {
            return .None
        }
    }

    .Some(params)
}

/// Splits a path into non-empty segments.
func splitPathSegments(path: String) -> Array[String] {
    var segments = Array[String]();
    for part in path.split("/") {
        if part.byteCount > 0 {
            segments.append(part.toOwned())
        }
    }
    segments
}
