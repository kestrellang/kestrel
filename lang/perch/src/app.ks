/// The application, route groups, and the shared route table.

module perch

import http.method.(HttpMethod)
import http.content.(Text)
import http.status.(StatusCode)
import std.network.(TcpListener)

// ============================================================================
// ENDPOINTS
// ============================================================================

/// An HTTP method plus a path relative to its group: `Get("/:id")`.
public protocol Endpoint {
    func method() -> HttpMethod
    func path() -> String
}

public struct Get: Endpoint {
    var route: String
    public init(path: String) { self.route = path }
    public init() { self.route = String() }
    public func method() -> HttpMethod = HttpMethod.Get
    public func path() -> String = self.route.clone()
}

public struct Post: Endpoint {
    var route: String
    public init(path: String) { self.route = path }
    public init() { self.route = String() }
    public func method() -> HttpMethod = HttpMethod.Post
    public func path() -> String = self.route.clone()
}

public struct Put: Endpoint {
    var route: String
    public init(path: String) { self.route = path }
    public init() { self.route = String() }
    public func method() -> HttpMethod = HttpMethod.Put
    public func path() -> String = self.route.clone()
}

public struct Patch: Endpoint {
    var route: String
    public init(path: String) { self.route = path }
    public init() { self.route = String() }
    public func method() -> HttpMethod = HttpMethod.Patch
    public func path() -> String = self.route.clone()
}

public struct Delete: Endpoint {
    var route: String
    public init(path: String) { self.route = path }
    public init() { self.route = String() }
    public func method() -> HttpMethod = HttpMethod.Delete
    public func path() -> String = self.route.clone()
}

// ============================================================================
// ROUTE TABLE
// ============================================================================

/// One registered route, its context and extractors already erased: the
/// handler takes the matched request and returns the response.
struct RouteEntry: Cloneable {
    var method: HttpMethod
    var segments: Array[String]
    var handler: escaping (Request) -> Response

    func clone() -> RouteEntry =
        RouteEntry(method: self.method, segments: self.segments.clone(), handler: self.handler)
}

/// Every route of an app. Groups and the app share ONE table (through an
/// `RcBox`), so registering on any of them — even through a `let` — adds
/// to the app that will `listen`.
struct RouteTable {
    var routes: Array[RouteEntry]

    /// The first route matching `method` and `segments`, and the values
    /// its `:param` segments bound (in pattern order, and by name).
    func find(method: HttpMethod, segments: Array[String]) -> (RouteEntry, Array[String], Dictionary[String, String])? {
        for route in self.routes {
            if route.method.toString() != method.toString() { continue }
            if let .Some((values, named)) = matchPath(segments, route.segments) {
                return .Some((route.clone(), values, named))
            }
        }
        .None
    }
}

/// Matches request segments against a pattern; `:name` binds any segment.
func matchPath(requestSegments: Array[String], patternSegments: Array[String]) -> (Array[String], Dictionary[String, String])? {
    if requestSegments.count != patternSegments.count {
        return .None
    }
    var values: Array[String] = [];
    var named: Dictionary[String, String] = [:];
    for i in 0..<patternSegments.count {
        let pattern = unsafe patternSegments(unchecked: i);
        let actual = unsafe requestSegments(unchecked: i);
        if pattern.starts(with: ":") {
            let name = pattern.asSlice().subslice(from: 1, to: pattern.bytes.count).toOwned();
            values.append(actual.clone());
            named.insert(name, actual.clone());
        } else if pattern != actual {
            return .None
        }
    }
    .Some((values, named))
}

/// Splits a path into its non-empty segments.
func splitPathSegments(path: String) -> Array[String] {
    var segments: Array[String] = [];
    for part in path.split(by: "/") {
        if part.bytes.count > 0 {
            segments.append(part.toOwned())
        }
    }
    segments
}

// ============================================================================
// GROUP
// ============================================================================

/// Routes under a shared path prefix whose handlers see a `Ctx`.
///
/// A group is a cheap handle: copies share the app's route table. `using`
/// returns a NEW group with a different context type; routes registered
/// on the original are unaffected.
public struct Group[Ctx]: Cloneable {
    var table: RcBox[RouteTable]
    var prefix: String
    /// Builds this group's context for a request: the app context, run
    /// through every middleware the group was derived with.
    var resolve: escaping (Request) -> MiddlewareResult[Ctx]

    public func clone() -> Group[Ctx] =
        Group[Ctx](table: self.table.clone(), prefix: self.prefix.clone(), resolve: self.resolve)

    /// A subgroup at `prefix` (relative to this group's) with the same context.
    public func group(prefix: String) -> Group[Ctx] =
        Group[Ctx](table: self.table.clone(), prefix: self.prefix + prefix, resolve: self.resolve)

    /// A group whose handlers run behind `middleware` and see its `Output`.
    public func using[M](middleware: M) -> Group[M.Output] where M: Middleware[Ctx] {
        let parent = self.resolve;
        let next: escaping (Request) -> MiddlewareResult[M.Output] = { (request: Request) in
            match parent(request) {
                .Continue(ctx) => middleware.handle(request, ctx),
                .Respond(response) => .Respond(response)
            }
        };
        Group[M.Output](table: self.table.clone(), prefix: self.prefix.clone(), resolve: next)
    }

    /// Registers `handler` for `endpoint`. Perch builds the context, then
    /// the input `I` from the request; a failure at either step is the
    /// response and the handler doesn't run.
    public func on[E, I, R](endpoint: E, handler: escaping (Ctx, I) -> R throws HttpError)
        where E: Endpoint, I: FromRequest, R: IntoResponse
    {
        let resolve = self.resolve;
        let erased: escaping (Request) -> Response = { (request: Request) in
            let ctx = match resolve(request) {
                .Continue(ctx) => ctx,
                .Respond(response) => return response
            };
            let input = match I.fromRequest(request) {
                .Ok(input) => input,
                .Err(error) => return error.intoResponse()
            };
            match handler(ctx, input) {
                .Ok(output) => output.intoResponse(),
                .Err(error) => error.intoResponse()
            }
        };
        let entry = RouteEntry(
            method: endpoint.method(),
            segments: splitPathSegments(self.prefix + endpoint.path()),
            handler: erased
        );
        // The table is shared by every handle, so write through the shared box.
        self.table.sharedMutRef().routes.append(entry);
    }
}

// ============================================================================
// APP
// ============================================================================

/// A Perch application. `Ctx` is the app-wide context (database handles,
/// config) every handler starts from; it is cloned once per request.
///
/// ```
/// let app = App(AppCtx(db: db)).using(Logger());
/// app.on(Post("/auth/login"), Auth.login);
///
/// let notes = app.group("/notes").using(AuthMiddleware());
/// notes.on(Get("/:id"), Notes.read);
///
/// try app.listen(8080);
/// ```
public struct App[Ctx]: Cloneable {
    var root: Group[Ctx]

    /// Creates an app with no routes whose handlers start from `context`.
    public init(context: Ctx) where Ctx: Cloneable {
        let resolve: escaping (Request) -> MiddlewareResult[Ctx] =
            { (request: Request) in .Continue(context.clone()) };
        self.root = Group[Ctx](
            table: RcBox(RouteTable(routes: [])),
            prefix: String(),
            resolve: resolve
        )
    }

    init(wrapping root: Group[Ctx]) {
        self.root = root
    }

    public func clone() -> App[Ctx] = App[Ctx](wrapping: self.root.clone())

    /// The app with `middleware` in front of every route registered on the
    /// result afterwards.
    public func using[M](middleware: M) -> App[M.Output] where M: Middleware[Ctx] =
        App[M.Output](wrapping: self.root.using(middleware))

    /// A route group at `prefix` with the app's context.
    public func group(prefix: String) -> Group[Ctx] = self.root.group(prefix)

    /// Registers `handler` for `endpoint` at the app root.
    public func on[E, I, R](endpoint: E, handler: escaping (Ctx, I) -> R throws HttpError)
        where E: Endpoint, I: FromRequest, R: IntoResponse
    {
        self.root.on(endpoint, handler)
    }

    /// Serves requests on `port` until accepting fails. Each connection
    /// handles one request (`Connection: close`).
    public func listen(port: UInt16) -> () throws ServerError {
        var listener = try TcpListener.bind(port);
        println("Perch listening on port \(port)");
        loop {
            // match instead of try: a `try` in a loop deferred
            // $try_value's deinit past the next accept() and closed the
            // new connection's fd (see perch 0.3).
            match listener.accept() {
                .Ok((stream, _)) => {
                    var stream = stream;
                    let response = match parseHttpRequest(stream) {
                        .Ok(request) => self.respond(to: request),
                        .Err(_) => Response(StatusCode.badRequest(), Text("Bad Request"))
                    };
                    // A client that hung up mid-response is its problem, not the server's.
                    let _sent = sendResponse(response, to: stream);
                },
                .Err(e) => throw ServerError(e.description())
            }
        }
    }

    /// The response to one request: what `listen` sends, without a socket.
    /// 404 when no route matches.
    public func respond(to request: Request) -> Response {
        guard let .Some((route, values, named)) =
            self.root.table.value.find(request.method, request.segments) else {
            return Response.notFound()
        }
        var matched = request;
        matched.pathValues = values;
        matched.pathParams = named;
        (route.handler)(matched)
    }
}
