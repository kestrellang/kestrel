// test: execution
// expect-exit: 0

// Regression: field-as-call lowering must substitute inherited generic type
// arguments through function-typed fields before dispatching to Array.subscript.

module test.field_subscript_function_type_substitution

struct Request {}

enum MiddlewareResult {
    case Continue(Request)
    case Respond(Int64)
}

struct Route[T]: Cloneable {
    var middleware: Array[(Request, T) -> MiddlewareResult]

    func clone() -> Route[T] {
        Route[T](middleware: self.middleware.clone())
    }
}

struct Router[T]: Cloneable {
    var routes: Array[Route[T]]

    func clone() -> Router[T] {
        Router[T](routes: self.routes.clone())
    }

    func touch() -> Int64 {
        var i: Int64 = 0;
        while i < self.routes.count {
            let route = self.routes(unchecked: i);
            var j: Int64 = 0;
            while j < route.middleware.count {
                let _ = route.middleware(unchecked: j);
                j = j + 1
            }
            i = i + 1
        }
        0
    }
}

@main
func main() -> lang.i64 {
    let router = Router[Int64](routes: Array[Route[Int64]]());
    router.touch().raw
}
