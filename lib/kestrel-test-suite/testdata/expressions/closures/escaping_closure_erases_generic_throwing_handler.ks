// test: execution
// stdlib: true
// expect-exit: 0
//
// A router stores routes as one erased closure type; `on[I]` wraps a generic
// throwing handler plus `I.extract` (a static call on the type param) in an
// escaping closure.

module Test

struct HttpError { var status: Int64 }

protocol FromRequest { static func extract(from raw: Int64) -> Self throws HttpError }

struct Id: FromRequest {
    var v: Int64
    static func extract(from raw: Int64) -> Id throws HttpError {
        if raw < 0 { return .Err(HttpError(status: 400)) }
        .Ok(Id(v: raw))
    }
}

struct Ctx { var base: Int64 }

// The router stores every route as one erased closure type.
struct Router {
    var routes: Array[escaping (Int64, Ctx) -> Int64]

    mutating func on[I](handler: escaping (Ctx, I) -> Int64 throws HttpError) where I: FromRequest {
        self.routes.append({ (raw: Int64, ctx: Ctx) in
            let input = match I.extract(from: raw) { .Ok(i) => i, .Err(e) => return e.status };
            match handler(ctx, input) { .Ok(v) => v, .Err(e) => e.status }
        })
    }
}

func read(ctx: Ctx, id: Id) -> Int64 throws HttpError {
    if id.v == 0 { return .Err(HttpError(status: 404)) }
    .Ok(ctx.base + id.v)
}

@main
func main() -> Int64 {
    var router = Router(routes: []);
    router.on(read);
    let ctx = Ctx(base: 200);
    let ok = (router.routes(0))(1, ctx);     // 201
    let bad = (router.routes(0))(-1, ctx);   // 400
    let missing = (router.routes(0))(0, ctx); // 404
    ok + bad + missing - 1005
}
