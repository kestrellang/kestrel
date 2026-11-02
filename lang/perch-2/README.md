# Perch 2

Web server framework for Kestrel: handlers are plain functions with typed
inputs and outputs, middleware upgrades the context type, and errors are
thrown.

```kestrel
import perch.(App, Logger, Get, Post, HttpError, Json, Path, Created)

let app = App(AppCtx(db: db)).using(Logger());
app.on(Post("/auth/login"), Auth.login);

let notes = app.group("/notes").using(AuthMiddleware());   // AppCtx → AuthCtx
notes.on(Get("/:id"), Notes.read);
notes.on(Put(), Notes.create);

try app.listen(8080);
```

## Handlers

Every handler is `(ctx, input) -> R throws HttpError`:

- **`ctx`** is the group's context — whatever its middleware produced.
- **`input`** conforms to `FromRequest`: `Json[T]` (`T: Deserialize`),
  `Path[T]` (`T: PathParameter`), `Query[T]` (`T: Deserialize`), `()`, or a
  tuple of up to four extractors. A failed extraction is the response
  (400/404/422) and the handler doesn't run. Destructure extractors right in
  the parameter: `Json { value: new }: Json[NewNote]`, or for a tuple
  `(Path { value: id }, Json { value: edit }): (Path[Int64], Json[Edit])`.
- **`R`** conforms to `IntoResponse`; the status lives in the type:
  `Json[T]` 200, `Created[R]` 201, `NoContent` 204, `String` 200 text,
  `Response` as-is.
- A thrown **`HttpError`** (`.notFound()`, `.unauthorized()`,
  `.conflict(msg)`, …) becomes a JSON `{"error": msg}` response.

```kestrel
static func read(ctx: AuthCtx, Path { value: id }: Path[Int64]) -> Json[Note] throws HttpError {
    guard let some note = try findNote(ctx.db, id) else {
        throw HttpError.notFound()
    }
    Json(note)
}
```

## Middleware

```kestrel
struct AuthMiddleware: Middleware[AppCtx] {
    type Output = AuthCtx;
    func handle(request: Request, ctx: AppCtx) -> MiddlewareResult[AuthCtx] { … }
}
```

`group.using(m)` returns a `Group[M.Output]`, so a handler that takes
`AuthCtx` can only be registered behind the middleware that produces one.
Middleware is stored once and borrowed; it need not be `Cloneable`.

## Sharing

`App` and `Group` are handles onto one route table: registering through a
`let` group adds to the app that `listen`s. `using` and `group` return new
handles; they never change the one they're called on.

## Testing

`app.respond(to: Request(.Get, url: "/notes/1", headers: h, body: ""))`
runs a request through routing, middleware, and the handler without a
socket. `tests/run.sh` builds and runs `tests/checks.ks`.

## Notes

Middleware runs only for matched routes, so `Logger` doesn't log 404s.
`Path[T]` reads the route's first `:param`; take `Path[(A, B)]` for two.
