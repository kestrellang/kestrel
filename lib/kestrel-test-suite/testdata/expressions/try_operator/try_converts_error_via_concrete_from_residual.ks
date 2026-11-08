// test: execution
// stdlib: true
// expect-exit: 0
//
// `try` on `Result[_, DbError]` inside a `-> T throws HttpError` function uses a
// second, concrete `FromResidual[DbError]` conformance on `Result[T, HttpError]`.
// Today adding that conformance breaks `try` program-wide, including inside
// stdlib io (".fromResidual not found on _").

module Test

struct DbError { var code: Int64 }
struct HttpError { var status: Int64 }

// A second FromResidual conformance alongside the built-in FromResidual[HttpError].
extend Result[T, HttpError]: FromResidual[DbError] {
    static func fromResidual(consuming residual: DbError) -> Result[T, HttpError] {
        .Err(HttpError(status: 500))
    }
}

func query() -> Int64 throws DbError { .Err(DbError(code: 7)) }

func handler() -> Int64 throws HttpError {
    let n = try query();
    .Ok(n)
}

@main
func main() -> Int64 {
    match handler() {
        .Ok(_) => 1,
        .Err(e) => e.status - 500
    }
}
