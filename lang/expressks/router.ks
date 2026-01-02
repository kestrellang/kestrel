module expressks.router

import std.collections.dictionary
import expressks.http
import expressks.middleware

// Type alias for route handlers
public type Handler = (Request) -> Response

public struct Router {
    private var routes: Dictionary[String, Handler]
    private var middlewares: Dictionary[String, any Middleware] // Simplification

    public init() {
        self.routes = Dictionary()
        self.middlewares = Dictionary()
    }

    public func get(path: String, handler: Handler) {
        self.addRoute(method: "GET", path: path, handler: handler)
    }

    public func post(path: String, handler: Handler) {
        self.addRoute(method: "POST", path: path, handler: handler)
    }

    private func addRoute(method: String, path: String, handler: Handler) {
        let key = method + ":" + path
        self.routes.insert(value: handler, for: key)
    }

    public func handle(req: Request) -> Response {
        let key = req.method + ":" + req.path
        match self.routes[key] {
            .Some(let handler) => handler(req),
            .None => .Text("404 Not Found")
        }
    }
    
    // TODO: Add proper middleware chain execution
}
