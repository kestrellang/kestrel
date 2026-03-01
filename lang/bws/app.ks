// Express-like HTTP application router
//
// Provides App for routing HTTP requests to handler functions.

module bws.app

import std.num.(Int64, Int32, UInt8, UInt16)
import std.result.(Result)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import std.net.socket.(TcpListener, TcpStream)
import std.net.libc
import bws.request.(HttpMethod, HttpRequest, parseRequest)
import bws.response.(HttpResponse, statusTextFor)
import std.io.error.(Error)
import std.io.stdio.(println)

// ============================================================================
// ROUTE
// ============================================================================

/// An internal route entry mapping a method + path to a handler.
struct Route {
    var method: HttpMethod
    var path: String
    var handler: (HttpRequest) -> HttpResponse
}

// ============================================================================
// APP
// ============================================================================

/// An Express-like HTTP application.
///
/// Register route handlers with onGet(), onPost(), etc., then call listen()
/// to start the server.
///
/// Example:
///     var app = App()
///     app.onGet("/", { (req: HttpRequest) in
///         var res = HttpResponse()
///         res.text("Hello, World!")
///         res
///     })
///     match app.listen(port: 8080) {
///         .Ok(_) => {},
///         .Err(e) => println("Error: " + e.description())
///     }
public struct App {
    var routes: Array[Route]

    /// Creates a new App with no routes.
    public init() {
        self.routes = Array[Route]()
    }

    /// Registers a handler for the given method and path.
    public mutating func route(method: HttpMethod, path: String, handler: (HttpRequest) -> HttpResponse) {
        self.routes.append(Route(method: method, path: path, handler: handler))
    }

    /// Registers a GET route handler.
    public mutating func onGet(path: String, handler: (HttpRequest) -> HttpResponse) {
        self.routes.append(Route(method: .Get, path: path, handler: handler))
    }

    /// Registers a POST route handler.
    public mutating func onPost(path: String, handler: (HttpRequest) -> HttpResponse) {
        self.routes.append(Route(method: .Post, path: path, handler: handler))
    }

    /// Registers a PUT route handler.
    public mutating func onPut(path: String, handler: (HttpRequest) -> HttpResponse) {
        self.routes.append(Route(method: .Put, path: path, handler: handler))
    }

    /// Registers a DELETE route handler.
    public mutating func onDelete(path: String, handler: (HttpRequest) -> HttpResponse) {
        self.routes.append(Route(method: .Delete, path: path, handler: handler))
    }

    /// Starts the server, listening on the given port.
    ///
    /// This blocks forever, accepting connections and dispatching requests.
    /// Each connection is handled synchronously. Responses include
    /// Connection: close, so each request uses a fresh connection.
    public func listen(port: UInt16) -> Result[(), Error] {
        var listener = try TcpListener.bind(port);

        let _ = println("Server listening on port " + intToStr(Int64(from: port)));

        loop {
            var stream = try listener.accept();
            let clientFd = stream.rawFd();

            match parseRequest(clientFd) {
                .Ok(req) => {
                    let resp = self.dispatch(req);
                    let _ = resp.send(to: clientFd);
                },
                .Err(_) => {
                    // Send 400 Bad Request for unparseable requests
                    var resp = HttpResponse();
                    resp.setStatus(400);
                    resp.text("Bad Request");
                    let _ = resp.send(to: clientFd);
                }
            }
            // stream dropped here, closing the connection
        }
    }

    /// Dispatches a request to the matching route handler.
    ///
    /// Returns 404 if no route matches the path, or 405 if the path
    /// matches but the method doesn't.
    func dispatch(req: HttpRequest) -> HttpResponse {
        var pathMatched = false;

        var i: Int64 = 0;
        while i < self.routes.count {
            let route = self.routes(unchecked: i);
            if route.path.equals(req.path) {
                pathMatched = true;
                match (route.method, req.method) {
                    (.Get, .Get) => return (route.handler)(req),
                    (.Post, .Post) => return (route.handler)(req),
                    (.Put, .Put) => return (route.handler)(req),
                    (.Delete, .Delete) => return (route.handler)(req),
                    (.Patch, .Patch) => return (route.handler)(req),
                    (.Head, .Head) => return (route.handler)(req),
                    (.Options, .Options) => return (route.handler)(req),
                    _ => {}
                }
            }
            i = i + 1
        }

        var resp = HttpResponse();
        if pathMatched {
            resp.setStatus(405);
            resp.text("Method Not Allowed")
        } else {
            resp.setStatus(404);
            resp.text("Not Found")
        }
        resp
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// Simple Int64 to String conversion for the port number display.
func intToStr(n: Int64) -> String {
    if n == 0 {
        return "0"
    }
    var result = String();
    var value = n;
    var digits = Array[UInt8]();
    while value > 0 {
        let digitValue = value % 10 + 48;
        let digit = UInt8(from: digitValue);
        digits.append(digit);
        value = value / 10
    }
    var i = digits.count - 1;
    while i >= 0 {
        result.appendByte(digits(unchecked: i));
        i = i - 1
    }
    result
}
