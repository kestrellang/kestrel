// test: execution
// stdlib: true
// expect-exit: 0
//
// Same as the concrete case, but the conversion is generic over any `E: Reportable`.
// Same-type residuals (HttpError → HttpError) must still pass through untouched.

module Test

protocol Reportable { func report() -> Int64 }

struct DbError: Reportable {
    var code: Int64
    func report() -> Int64 { self.code }
}
struct HttpError { var status: Int64 }

// Generic: any other Reportable error becomes a 500.
extend[E] Result[T, HttpError]: FromResidual[E] where E: Reportable {
    static func fromResidual(consuming residual: E) -> Result[T, HttpError] {
        .Err(HttpError(status: 500))
    }
}

func query() -> Int64 throws DbError { .Err(DbError(code: 7)) }

func notFound() -> Int64 throws HttpError { .Err(HttpError(status: 404)) }

func handler(which: Int64) -> Int64 throws HttpError {
    if which == 0 { let n = try query(); return .Ok(n) }
    let m = try notFound();   // same-type residual must still pass through untouched
    .Ok(m)
}

@main
func main() -> Int64 {
    let a = match handler(0) { .Ok(_) => 1, .Err(e) => e.status - 500 };
    let b = match handler(1) { .Ok(_) => 1, .Err(e) => e.status - 404 };
    a + b
}
