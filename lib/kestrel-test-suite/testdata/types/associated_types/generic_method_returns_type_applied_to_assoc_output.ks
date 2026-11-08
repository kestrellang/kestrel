// test: execution
// stdlib: true
// expect-exit: 0
//
// `func using[M](m: M) -> Group[M.Output] where M: Middleware[Ctx]` changes the
// group's context type, and `Logger[C]()` infers C from the expected `Middleware[Ctx]`.

module Test

protocol Middleware[In] {
    type Output
    func handle(ctx: In) -> Output
}

struct AppCtx { var n: Int64 }
struct AuthCtx { var n: Int64; var user: Int64 }

struct Auth: Middleware[AppCtx] {
    type Output = AuthCtx
    func handle(ctx: AppCtx) -> AuthCtx { AuthCtx(n: ctx.n, user: 7) }
}

// Generic identity middleware; C must be inferred from `using`'s expected type.
struct Logger[C]: Middleware[C] {
    type Output = C
    func handle(ctx: C) -> C { ctx }
}

struct Group[Ctx] {
    var label: Int64
    func using[M](middleware: M) -> Group[M.Output] where M: Middleware[Ctx] {
        Group[M.Output](label: self.label + 1)
    }
}

@main
func main() -> Int64 {
    let g: Group[AppCtx] = Group(label: 0);
    let logged = g.using(Logger());          // Group[AppCtx]
    let authed = logged.using(Auth());       // Group[AuthCtx]
    let again: Group[AuthCtx] = authed.using(Logger());
    again.label - 3
}
