/// Perch web application — top-level entry point for building and
/// running an HTTP server.

module perch.app

import http.method.(HttpMethod)
import perch.request.(Request)
import perch.response.(Response)
import perch.context.(MiddlewareResult, Middleware)
import perch.router.(Router, GroupBuilder, Routes)
import perch.parse.(parseHttpRequest)
import perch.send.(sendResponse)
import http.content.(Text)
import std.io.error.(IoError)

/// A Perch web application parameterized by a context type `T`.
///
/// The context holds app-wide state — database connections, config,
/// caches — anything your handlers need. It is created once at startup
/// and passed to every handler and middleware by value.
///
/// # Examples
///
/// ```
/// struct Ctx {}
///
/// var app = App(Ctx());
/// app.use(Logger[Ctx]());
/// app.route(get: "/", { (req: Request, ctx: Ctx) in
///     Response.ok(Text("Hello, world!"))
/// });
/// let _ = app.listen(8080);
/// ```
public struct App[T]: Cloneable, Routes[T] where T: Cloneable {
    var router: Router[T]
    var context: T

    /// Creates a new app with the given context and no routes.
    public init(context: T) {
        self.router = Router[T]();
        self.context = context
    }

    public func clone() -> App[T] {
        var copy = App[T](self.context.clone());
        copy.router = self.router.clone();
        copy
    }

    // ========================================================================
    // MIDDLEWARE
    // ========================================================================

    /// Adds global middleware that runs on every request.
    public mutating func use[M](middleware: M) where M: Middleware[T] {
        self.router.use(middleware)
    }

    // ========================================================================
    // ROUTE REGISTRATION
    // ========================================================================

    public mutating func addRoute(method: HttpMethod, path: String, handler: (Request, T) -> Response) {
        self.router.addRoute(method, path, handler)
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
    /// Blocks forever, accepting connections and dispatching requests
    /// one at a time. Each connection is closed after the response is
    /// sent (`Connection: close`).
    ///
    /// # Errors
    ///
    /// Returns `IoError` if the TCP bind or accept fails (e.g. port
    /// already in use, permission denied).
    public func listen(port: UInt16) -> Result[(), IoError] {
        var listener = try TcpListener.bind(port);
        let _ = println("Perch listening on port \(port)");

        loop {
            var stream = try listener.accept();
            let fd = stream.rawFd();

            match parseHttpRequest(fd) {
                .Ok(request) => {
                    let response = self.dispatch(request);
                    let _ = sendResponse(response, to: fd);
                },
                .Err(_) => {
                    let badReq = Response.badRequest(Text("Bad Request"));
                    let _ = sendResponse(badReq, to: fd);
                }
            }
        }
    }

    // ========================================================================
    // DISPATCH
    // ========================================================================

    /// Dispatches a request through the middleware pipeline and route handler.
    ///
    /// Execution order: global middleware → group middleware → handler.
    /// The first middleware that returns `.Respond` short-circuits the
    /// chain. Returns 404 if no route matches.
    func dispatch(request: Request) -> Response {
        var req = request;
        for mw in self.router.globalMiddleware {
            match mw(req, self.context) {
                .Continue(enriched) => { req = enriched },
                .Respond(response) => { return response }
            }
        }

        guard let .Some(matchResult) = self.router.findRoute(req.method, req.segments) else {
            return Response.notFound();
        }

        req.pathParams = matchResult.params;

        for mw in matchResult.groupMiddleware {
            match mw(req, self.context) {
                .Continue(enriched) => { req = enriched },
                .Respond(response) => { return response }
            }
        }

        for mw in matchResult.routeMiddleware {
            match mw(req, self.context) {
                .Continue(enriched) => { req = enriched },
                .Respond(response) => { return response }
            }
        }

        (matchResult.handler)(req, self.context)
    }
}
