# Perch

Web server framework for Kestrel with routing, middleware, and parameterized context.

## Installation

```toml
[dependencies]
kestrel/perch = "0.1.0"
```

## Usage

```kestrel
var app = App[String]("Hello, World!")

app.onGet("/", func(req: Request, ctx: String) -> Response {
    return Response.ok(body: ctx)
})

app.onGet("/users/:id", func(req: Request, ctx: String) -> Response {
    let id = req.params("id")
    return Response.ok(body: "User " + id)
})

app.onPost("/users", func(req: Request, ctx: String) -> Response {
    return Response.created(body: "Created")
})

let _ = app.listen(port: 8080)
```

## Key Types

- **App[T]** - web application with typed context
- **Request** - incoming HTTP request with method, path segments, headers, body, and route params
- **Response** - HTTP response builder
- **GroupBuilder[T]** - route group with shared prefix and middleware

## Routing

Register handlers with `onGet`, `onPost`, `onPut`, `onDelete`, `onPatch`:

```kestrel
app.onGet("/api/users", handler)
app.onGet("/api/users/:id", handler)  // :id is a path parameter
```

## Route Groups

```kestrel
var api = GroupBuilder[String]("/api")
api.onGet("/users", listUsers)
api.onPost("/users", createUser)
app.addGroup(api)
```

## Middleware

```kestrel
app.use(func(req: Request, ctx: String) -> MiddlewareResult {
    // Return .Continue(req) to proceed, or .Respond(response) to short-circuit
    return MiddlewareResult.Continue(req)
})
```
