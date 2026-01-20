// Router - route matching and middleware chain execution
//
// The Router handles HTTP method routing with path parameter extraction
// and middleware chain execution.

module expressks.router;

import std.collections.dictionary;
import std.collections.array;
import expressks.http.(Request, Response, HttpMethod, NotFound);
import expressks.middleware.(Middleware, Handler);
import expressks.internal.path.PathPattern;

// A registered route
struct Route {
    let method: HttpMethod;
    let pattern: PathPattern;
    let handler: Handler;
}

// Middleware registration with optional path prefix
struct MiddlewareEntry {
    let pathPrefix: Optional[String];
    let middleware: any Middleware;
}

// Route match result
public struct RouteMatch {
    public let handler: Handler;
    public let params: Dictionary[String, String];

    public init(handler: Handler, params: Dictionary[String, String]) {
        self.handler = handler;
        self.params = params;
    }
}

// The Router struct
public struct Router {
    private var routes: Array[Route];
    private var middlewares: Array[MiddlewareEntry];
    private let prefix: String;

    public init() {
        self.routes = Array[Route]();
        self.middlewares = Array[MiddlewareEntry]();
        self.prefix = "";
    }

    public init(prefix: String) {
        self.routes = Array[Route]();
        self.middlewares = Array[MiddlewareEntry]();
        self.prefix = prefix;
    }

    // GET route
    public func get(path: String, handler: Handler) {
        self.addRoute(method: .Get, path: path, handler: handler);
    }

    // POST route
    public func post(path: String, handler: Handler) {
        self.addRoute(method: .Post, path: path, handler: handler);
    }

    // PUT route
    public func put(path: String, handler: Handler) {
        self.addRoute(method: .Put, path: path, handler: handler);
    }

    // DELETE route
    public func delete(path: String, handler: Handler) {
        self.addRoute(method: .Delete, path: path, handler: handler);
    }

    // PATCH route
    public func patch(path: String, handler: Handler) {
        self.addRoute(method: .Patch, path: path, handler: handler);
    }

    // HEAD route
    public func head(path: String, handler: Handler) {
        self.addRoute(method: .Head, path: path, handler: handler);
    }

    // OPTIONS route
    public func options(path: String, handler: Handler) {
        self.addRoute(method: .Options, path: path, handler: handler);
    }

    // Add route with any method
    public func route(method: HttpMethod, path: String, handler: Handler) {
        self.addRoute(method: method, path: path, handler: handler);
    }

    // Internal: add a route
    private func addRoute(method: HttpMethod, path: String, handler: Handler) {
        let fullPath = self.prefix + path;
        let pattern = PathPattern.compile(fullPath);
        self.routes.append(Route(method: method, pattern: pattern, handler: handler));
    }

    // Add global middleware
    public func use(middleware: any Middleware) {
        self.middlewares.append(MiddlewareEntry(pathPrefix: .None, middleware: middleware));
    }

    // Add path-scoped middleware
    public func use(path: String, middleware: any Middleware) {
        let fullPath = self.prefix + path;
        self.middlewares.append(MiddlewareEntry(pathPrefix: .Some(fullPath), middleware: middleware));
    }

    // Find matching route
    public func findRoute(method: HttpMethod, path: String) -> Optional[RouteMatch] {
        for route in self.routes {
            if route.method.equals(method) {
                match route.pattern.match(path: path) {
                    .Some(let params) => {
                        return .Some(RouteMatch(handler: route.handler, params: params));
                    },
                    .None => {}
                }
            }
        }
        .None
    }

    // Get applicable middlewares for a path
    public func getMiddlewares(path: String) -> Array[any Middleware] {
        var result = Array[any Middleware]();

        for entry in self.middlewares {
            match entry.pathPrefix {
                .None => {
                    // Global middleware - always applies
                    result.append(entry.middleware);
                },
                .Some(let prefix) => {
                    // Path-scoped middleware - check if path starts with prefix
                    if path.starts(with: prefix) {
                        result.append(entry.middleware);
                    }
                }
            }
        }

        result
    }

    // Handle a request with middleware chain
    public func handle(req: Request) -> Response {
        // Find matching route
        let routeMatch = self.findRoute(method: req.method, path: req.path);

        match routeMatch {
            .None => NotFound(),
            .Some(let matchResult) => {
                // Update request with path params
                var request = req;
                request.paramsMap = matchResult.params;

                // Get applicable middlewares
                let middlewares = self.getMiddlewares(path: req.path);

                if middlewares.count == 0 {
                    // No middleware, call handler directly
                    matchResult.handler(request)
                } else {
                    // Execute middleware chain
                    self.executeChain(
                        middlewares: middlewares,
                        index: 0,
                        request: request,
                        finalHandler: matchResult.handler
                    )
                }
            }
        }
    }

    // Execute middleware chain recursively
    private func executeChain(
        middlewares: Array[any Middleware],
        index: Int,
        request: Request,
        finalHandler: Handler
    ) -> Response {
        if index >= middlewares.count {
            // All middleware executed, call final handler
            return finalHandler(request);
        }

        let middleware = middlewares(unchecked: index);

        // Create next function that continues the chain
        let next: Handler = { (req) in
            self.executeChain(
                middlewares: middlewares,
                index: index + 1,
                request: req,
                finalHandler: finalHandler
            )
        };

        // Call middleware with next function
        middleware.handle(req: request, next: next)
    }

    // Mount another router (copy its routes and middlewares)
    public func mount(router: Router) {
        for route in router.routes {
            self.routes.append(route);
        }
        for entry in router.middlewares {
            self.middlewares.append(entry);
        }
    }

    // Get the prefix for this router
    public func getPrefix() -> String {
        self.prefix
    }

    // Get route count
    public func routeCount() -> Int {
        self.routes.count
    }

    // Get middleware count
    public func middlewareCount() -> Int {
        self.middlewares.count
    }
}
