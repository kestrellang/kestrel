module scratch.perch_unchecked_repro

struct Request {}

enum MiddlewareResult {
    case Continue(Request)
    case Respond(Int64)
}

struct Route[T] {
    var middleware: Array[(Request, T) -> MiddlewareResult]
}

struct Router[T] {
    var routes: Array[Route[T]]

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

func main() -> Int64 {
    let r = Router[Int64](routes: Array[Route[Int64]]());
    r.touch()
}
