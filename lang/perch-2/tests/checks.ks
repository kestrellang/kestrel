// perch-2 checks: requests run through `App.respond(to:)` — routing,
// typed middleware, extractors, and response types — with no socket.
// Prints one PASS/FAIL line per check.

module perch.tests

import perch.(App, Get, Post, Put, Delete, Request, Response, HttpError, Json, Path, Query, Created, NoContent, Middleware, MiddlewareResult, Logger)
import http.method.(HttpMethod)
import http.headers.(Headers)
import quill.value.(Value)
import quill.serialize.(Serialize)
import quill.deserialize.(Deserialize, extractString, extractInt)
import quill.error.(SerializeError, DeserializeError)

// ---------------------------------------------------------------------------
// Contexts and middleware
// ---------------------------------------------------------------------------

struct AppCtx: Cloneable {
    var greeting: String
    func clone() -> AppCtx = AppCtx(greeting: self.greeting.clone())
}

struct AuthCtx {
    var greeting: String
    var userId: Int64
}

// `Authorization: user-<n>` authenticates as user n. Not Cloneable — Perch
// stores the middleware once and borrows it.
struct Auth: Middleware[AppCtx] {
    type Output = AuthCtx
    func handle(request: Request, ctx: AppCtx) -> MiddlewareResult[AuthCtx] {
        guard let .Some(header) = request.header("Authorization") else {
            return .Respond(Response.unauthorized())
        }
        guard header.starts(with: "user-") else {
            return .Respond(Response.unauthorized())
        }
        let rest = header.asSlice().subslice(from: 5, to: header.bytes.count).toOwned();
        guard let .Some(id) = Int64(parsing: rest) else {
            return .Respond(Response.unauthorized())
        }
        .Continue(AuthCtx(greeting: ctx.greeting.clone(), userId: id))
    }
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

struct Note: Serialize {
    var id: Int64
    var title: String
    var owner: Int64

    func toValue() -> Value throws SerializeError {
        var object: Dictionary[String, Value] = [:];
        object.insert("id", .Int(self.id));
        object.insert("title", .Str(self.title.clone()));
        object.insert("owner", .Int(self.owner));
        .Obj(object)
    }
}

struct Token: Serialize {
    var token: String

    func toValue() -> Value throws SerializeError {
        var object: Dictionary[String, Value] = [:];
        object.insert("token", .Str(self.token.clone()));
        .Obj(object)
    }
}

struct NewNote: Deserialize {
    var title: String
    static func fromValue(value: Value) -> NewNote throws DeserializeError {
        let title = try extractString(from: value, "title");
        NewNote(title: title)
    }
}

struct Page: Deserialize {
    var page: Int64
    static func fromValue(value: Value) -> Page throws DeserializeError {
        guard let .Some(.Str(raw)) = value.value(for: "page") else { return Page(page: 1) }
        Page(page: Int64(parsing: raw) ?? 1)
    }
}

// ---------------------------------------------------------------------------
// Handlers: `(ctx, input) -> R throws HttpError`, as static methods.
// ---------------------------------------------------------------------------

struct Handlers {
    static func hello(ctx: AppCtx, input: ()) -> String throws HttpError = ctx.greeting.clone()

    static func list(ctx: AuthCtx, Query { value: page }: Query[Page]) -> String throws HttpError {
        "user \(ctx.userId) page \(page.page)"
    }

    static func read(ctx: AuthCtx, Path { value: id }: Path[Int64]) -> Json[Note] throws HttpError {
        if id == 404 { throw HttpError.notFound() }
        Json(Note(id: id, title: "note", owner: ctx.userId))
    }

    static func create(ctx: AuthCtx, Json { value: new }: Json[NewNote]) -> Created[Json[Note]] throws HttpError {
        Created(Json(Note(id: 1, title: new.title, owner: ctx.userId)))
    }

    static func rename(ctx: AuthCtx, (Path { value: id }, Json { value: edit }): (Path[Int64], Json[NewNote])) -> Json[Note] throws HttpError {
        Json(Note(id: id, title: edit.title, owner: ctx.userId))
    }

    static func pair(ctx: AppCtx, Path { value: (id, name) }: Path[(Int64, String)]) -> String throws HttpError {
        "\(name)#\(id)"
    }

    static func token(ctx: AppCtx, input: ()) -> Json[Token] throws HttpError = Json(Token(token: "abc"))

    static func remove(ctx: AuthCtx, Path { value: id }: Path[Int64]) -> NoContent throws HttpError {
        if id < 0 { throw HttpError.conflict("negative id") }
        NoContent()
    }
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

func send(app: App[AppCtx], method: HttpMethod, url: String, auth: String?, body: String) -> Response {
    var headers = Headers();
    if let .Some(token) = auth {
        headers.setValue("Authorization", token)
    }
    app.respond(to: Request(method, url: url, headers: headers, body: body))
}

func check(name: String, response: Response, status: Int64, body: String) {
    if response.status.code == status and response.bodyContent == body {
        println("PASS \(name)")
    } else {
        println("FAIL \(name): got \(response.status.code) \(response.bodyContent), want \(status) \(body)")
    }
}

// Checks a JSON object body field by field: quill objects are hash maps, so
// key order isn't fixed.
func checkJson(name: String, response: Response, status: Int64, fields: Array[String]) {
    var missing: Array[String] = [];
    for field in fields {
        if response.bodyContent.firstIndex(of: field) == .None { missing.append(field.clone()) }
    }
    if response.status.code == status and missing.isEmpty {
        println("PASS \(name)")
    } else {
        println("FAIL \(name): got \(response.status.code) \(response.bodyContent), want \(status), missing \(missing.count) fields")
    }
}

@main
func main() -> Int64 {
    let app = App(AppCtx(greeting: "hi")).using(Logger());
    app.on(Get("/"), Handlers.hello);
    app.on(Get("/token"), Handlers.token);
    app.on(Get("/pair/:id/:name"), Handlers.pair);

    let notes = app.group("/notes").using(Auth());
    notes.on(Get(), Handlers.list);
    notes.on(Get("/:id"), Handlers.read);
    notes.on(Put(), Handlers.create);
    notes.on(Post("/:id"), Handlers.rename);
    notes.on(Delete("/:id"), Handlers.remove);

    let user = Optional.Some("user-7");

    check("unit input, String response", send(app, .Get, "/", .None, ""), 200, "hi");
    check("Json of a response struct", send(app, .Get, "/token", .None, ""), 200, "{\"token\":\"abc\"}");
    check("Path of a tuple", send(app, .Get, "/pair/4/kes", .None, ""), 200, "kes#4");
    check("Path tuple element that doesn't parse is 404", send(app, .Get, "/pair/x/kes", .None, ""), 404,
        "{\"error\":\"Not Found\"}");
    check("no route is 404", send(app, .Get, "/nope", .None, ""), 404, "Not Found");
    check("middleware short-circuits", send(app, .Get, "/notes", .None, ""), 401, "Unauthorized");
    check("query extractor + upgraded ctx", send(app, .Get, "/notes?page=3", user, ""), 200, "user 7 page 3");
    check("query default", send(app, .Get, "/notes", user, ""), 200, "user 7 page 1");
    checkJson("path extractor + Json response", send(app, .Get, "/notes/5", user, ""), 200,
        ["\"id\":5", "\"owner\":7", "\"title\":\"note\""]);
    check("unparsable path segment is 404", send(app, .Get, "/notes/abc", user, ""), 404,
        "{\"error\":\"Not Found\"}");
    check("thrown HttpError", send(app, .Get, "/notes/404", user, ""), 404, "{\"error\":\"Not Found\"}");
    checkJson("Json body + Created", send(app, .Put, "/notes", user, "{\"title\":\"new\"}"), 201,
        ["\"id\":1", "\"owner\":7", "\"title\":\"new\""]);
    check("invalid JSON is 400", send(app, .Put, "/notes", user, "{"), 400,
        "{\"error\":\"Invalid JSON: JSON parse error at offset 1: unexpected end of input in object\"}");
    checkJson("tuple input", send(app, .Post, "/notes/9", user, "{\"title\":\"t\"}"), 200,
        ["\"id\":9", "\"owner\":7", "\"title\":\"t\""]);
    check("JSON of the wrong shape is 422", send(app, .Put, "/notes", user, "{\"name\":\"x\"}"), 422,
        "{\"error\":\"deserialize error: missing key: title\"}");
    check("NoContent", send(app, .Delete, "/notes/2", user, ""), 204, "");
    check("conflict", send(app, .Delete, "/notes/-1", user, ""), 409, "{\"error\":\"negative id\"}");
    0
}
