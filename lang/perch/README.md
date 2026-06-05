# Perch

Web server framework for Kestrel with routing, middleware, and parameterized context.

## Installation

```toml
[dependencies]
kestrel/perch = "0.2.1"
```

## Usage

```kestrel
var app = App[String]("Hello, World!")

app.route(get: "/", func(req: Request, ctx: String) -> Response {
    return Response.ok(Text(ctx))
})

app.route(get: "/users/:id", func(req: Request, ctx: String) -> Response {
    let id = req.param("id")
    return Response.ok(Text("User " + id))
})

app.route(post: "/users", func(req: Request, ctx: String) -> Response {
    return Response.created(JsonBody(req.body))
})

let _ = app.listen(port: 8080)
```

## Key Types

- **App[T]** - web application with typed context
- **Request** - incoming HTTP request with method, path segments, headers, body, and route params
- **Response** - HTTP response builder with Content-based factories
- **GroupBuilder[T]** - route group with shared prefix and middleware
- **Content** - protocol for response/request body types (Text, JsonBody, Form, Bytes)
- **Middleware[T]** - protocol for request processing middleware

## Routing

Register handlers with `route(get:)`, `route(post:)`, `route(put:)`, `route(delete:)`, `route(patch:)`:

```kestrel
app.route(get: "/api/users", handler)
app.route(get: "/api/users/:id", handler)  // :id is a path parameter
```

All types that conform to `Routes[T]` get these methods automatically — just implement `addRoute`.

## Route Groups

```kestrel
var api = GroupBuilder[String]("/api")
api.route(get: "/users", listUsers)
api.route(post: "/users", createUser)
app.addGroup(api)
```

## Middleware

Implement the `Middleware[T]` protocol to create reusable middleware:

```kestrel
struct AuthCheck[T]: Middleware[T] {
    func handle(request: Request, ctx: T) -> MiddlewareResult {
        match request.header("Authorization") {
            .Some(_) => .Continue(request),
            .None => .Respond(Response.unauthorized())
        }
    }

    func clone() -> AuthCheck[T] { AuthCheck[T]() }
}

app.use(AuthCheck[String]())
```

## Response Content

Responses use the `Content` protocol for body data:

```kestrel
Response.ok(Text("Hello"))
Response.ok(JsonBody(payload))
Response.created(Form([("key", "value")]))
Response.badRequest(Text("Missing field"))
```
