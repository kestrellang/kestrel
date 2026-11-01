// test: execution
// stdlib: true
// expect-exit: 0
//
// `on[I, R](handler: (Ctx, I) -> R) where I: FromRequest, R: IntoResponse` infers
// I and R from a named function's signature alone.

module Test

protocol FromRequest { static func extract(from raw: Int64) -> Self }
protocol IntoResponse { func intoResponse() -> Int64 }

struct Path: FromRequest {
    var v: Int64
    static func extract(from raw: Int64) -> Path { Path(v: raw) }
}
struct Json: IntoResponse {
    var v: Int64
    func intoResponse() -> Int64 { self.v }
}

struct Ctx { var base: Int64 }

func read(ctx: Ctx, path: Path) -> Json { Json(v: ctx.base + path.v) }

// I and R must be inferred from the handler's signature alone.
func on[I, R](handler: (Ctx, I) -> R, raw: Int64) -> Int64 where I: FromRequest, R: IntoResponse {
    handler(Ctx(base: 40), I.extract(from: raw)).intoResponse()
}

@main
func main() -> Int64 {
    on(read, 2) - 42
}
