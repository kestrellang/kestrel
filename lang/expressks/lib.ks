// ExpressKS - Express.js-like web framework for Kestrel
//
// A simple, flexible web server library with:
// - HTTP method routing (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
// - Path parameters (/users/:id)
// - Query string parsing (?key=value)
// - Middleware support with full chain execution
// - Generic JSON serialization via Serialize protocol
//
// Usage:
// ```kestrel
// import expressks;
//
// func main() {
//     let app = expressks.createApp();
//
//     app.get(path: "/") { (req) in
//         Html("<h1>Hello Kestrel!</h1>")
//     };
//
//     app.get(path: "/users/:id") { (req) in
//         let userId = req.params("id").unwrap();
//         Json(getUserById(userId))
//     };
//
//     app.use(LoggingMiddleware());
//
//     app.listen(port: 8080);
// }
// ```

module expressks;

// Re-export HTTP types
public import expressks.http.(
    HttpMethod,
    HttpStatus,
    Request,
    Response,
    Html,
    Json,
    JsonRaw,
    Text,
    Redirect,
    NotFound,
    BadRequest,
    Unauthorized,
    Forbidden,
    InternalError,
    Empty,
    NoContent
);

// Re-export Router
public import expressks.router.Router;

// Re-export Middleware protocol and built-in middlewares
public import expressks.middleware.(
    Handler,
    Middleware,
    LoggingMiddleware,
    CorsMiddleware,
    AuthMiddleware,
    RequestIdMiddleware,
    RateLimitMiddleware,
    BodyLimitMiddleware,
    MethodOverrideMiddleware,
    ComposedMiddleware
);

// Re-export Server
public import expressks.server.(Server, ServerConfig);

// The main App struct - a convenient wrapper around Router + Server
public struct App {
    public var router: Router;

    public init() {
        self.router = Router();
    }

    // HTTP method route handlers

    // GET route
    public func get(path: String, handler: Handler) {
        self.router.get(path: path, handler: handler);
    }

    // POST route
    public func post(path: String, handler: Handler) {
        self.router.post(path: path, handler: handler);
    }

    // PUT route
    public func put(path: String, handler: Handler) {
        self.router.put(path: path, handler: handler);
    }

    // DELETE route
    public func delete(path: String, handler: Handler) {
        self.router.delete(path: path, handler: handler);
    }

    // PATCH route
    public func patch(path: String, handler: Handler) {
        self.router.patch(path: path, handler: handler);
    }

    // HEAD route
    public func head(path: String, handler: Handler) {
        self.router.head(path: path, handler: handler);
    }

    // OPTIONS route
    public func options(path: String, handler: Handler) {
        self.router.options(path: path, handler: handler);
    }

    // Middleware

    // Add global middleware
    public func use(middleware: any Middleware) {
        self.router.use(middleware: middleware);
    }

    // Add path-scoped middleware
    public func use(path: String, middleware: any Middleware) {
        self.router.use(path: path, middleware: middleware);
    }

    // Sub-routers

    // Create a sub-router with a path prefix
    public func group(prefix: String) -> Router {
        let subRouter = Router(prefix: prefix);
        // Note: routes need to be mounted after adding them
        subRouter
    }

    // Mount a sub-router
    public func mount(router: Router) {
        self.router.mount(router: router);
    }

    // Server

    // Start the server on the specified port
    public func listen(port: Int) {
        let server = Server(router: self.router);
        server.listen(port: port);
    }

    // Start the server with custom configuration
    public func listen(config: ServerConfig) {
        let server = Server(router: self.router, config: config);
        server.listen(port: config.port);
    }
}

// Factory function to create a new App
public func createApp() -> App {
    App()
}
