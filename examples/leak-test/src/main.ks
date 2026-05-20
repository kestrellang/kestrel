// Minimal Perch app for leak measurement.
// Handler returns a pre-cached string — no allocations in user code.

module leak_test.main

import perch.app.(App)
import perch.request.(Request)
import perch.response.(Response)
import http.content.(Text)

struct Ctx: Cloneable {
    var cached: String

    func clone() -> Ctx {
        Ctx(cached: self.cached.clone())
    }
}

func main() {
    var app = App(Ctx(cached: "OK"));
    app.route(get: "/", { (req: Request, ctx: Ctx) in
        Response.ok(Text(ctx.cached))
    });
    let _ = app.listen(8091);
}
