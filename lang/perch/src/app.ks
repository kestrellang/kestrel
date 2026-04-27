// Perch web application
//
// Top-level entry point for building and running a web server.

module perch.app

import http.method.(HttpMethod)
import perch.request.(Request)
import perch.response.(Response)
import perch.context.(MiddlewareResult)
import perch.router.(Router, GroupBuilder)
import perch.parse.(parseHttpRequest)
import perch.send.(sendResponse)
import std.io.error.(IoError)

/// A Perch web application parameterized by an app context type.
///
/// The context holds app-wide state like database connections, config,
/// caches — anything your handlers need. It's created once at startup
/// and passed to every handler and middleware.
public struct App[T] {
    var router: Router[T]
    var context: T

    /// Creates a new app with the given context and no routes.
    public init(context: T) {
        self.router = Router[T]();
        self.context = context
    }

    // ========================================================================
    // MIDDLEWARE
    // ========================================================================

    /// Adds global middleware that runs on every request.
    public mutating func use(mw: (Request, T) -> MiddlewareResult) {
        self.router.use(mw)
    }

    // ========================================================================
    // ROUTE REGISTRATION
    // ========================================================================

    /// Registers a GET route.
    public mutating func onGet(path: String, handler: (Request, T) -> Response) {
        self.router.onGet(path, handler)
    }

    /// Registers a POST route.
    public mutating func onPost(path: String, handler: (Request, T) -> Response) {
        self.router.onPost(path, handler)
    }

    /// Registers a PUT route.
    public mutating func onPut(path: String, handler: (Request, T) -> Response) {
        self.router.onPut(path, handler)
    }

    /// Registers a DELETE route.
    public mutating func onDelete(path: String, handler: (Request, T) -> Response) {
        self.router.onDelete(path, handler)
    }

    /// Registers a PATCH route.
    public mutating func onPatch(path: String, handler: (Request, T) -> Response) {
        self.router.onPatch(path, handler)
    }

    /// Adds a route group with shared prefix and middleware.
    public mutating func addGroup(group: GroupBuilder[T]) {
        self.router.addGroup(group)
    }

    // ========================================================================
    // SERVER
    // ========================================================================

    /// Starts the server, listening on the given port.
    ///
    /// This blocks forever, accepting connections and dispatching requests.
    /// Each connection is handled synchronously with Connection: close.
    public func listen(port: UInt16) -> Result[(), IoError] {
        var listener = try TcpListener.bind(port);
        let _ = println("Perch listening on port " + Int64(from: port).format());

        loop {
            var stream = try listener.accept();
            let fd = stream.rawFd();

            match parseHttpRequest(fd) {
                .Ok(request) => {
                    let response = self.dispatch(request);
                    let _ = sendResponse(response, to: fd);
                },
                .Err(_) => {
                    let _ = sendResponse(Response.badRequest("Bad Request"), to: fd);
                }
            }
        }
    }

    // ========================================================================
    // DISPATCH
    // ========================================================================

    /// Dispatches a request through the middleware pipeline and route handler.
    func dispatch(request: Request) -> Response {
        var req = request;

        // 1. Run global middleware
        for mw in self.router.globalMiddleware {
            match mw(req, self.context) {
                .Continue(enriched) => { req = enriched },
                .Respond(response) => { return response }
            }
        }

        // 2. Find matching route
        match self.router.findRoute(req.method, req.segments) {
            .Some(matchResult) => {
                // Set path params on request
                req = req.withPathParams(matchResult.params);

                // Run group middleware
                for mw in matchResult.groupMiddleware {
                    match mw(req, self.context) {
                        .Continue(enriched) => { req = enriched },
                        .Respond(response) => { return response }
                    }
                }

                // Run route middleware
                for mw in matchResult.routeMiddleware {
                    match mw(req, self.context) {
                        .Continue(enriched) => { req = enriched },
                        .Respond(response) => { return response }
                    }
                }

                // Run handler
                (matchResult.handler)(req, self.context)
            },
            .None => {
                Response.notFound()
            }
        }
    }
}
