// HTMX Counter Example
//
// A simple web counter with increment, decrement, and reset buttons.
// Uses Perch web framework and htmx for interactive updates.
//
// State is passed via query parameters since Kestrel doesn't yet
// support mutable shared state across requests.

module counter.main

import perch.app.(App)
import perch.request.(Request)
import perch.response.(Response)
import plume.plume.(Template)

struct Ctx: Cloneable {
    var x: Int64

    func clone() -> Ctx {
        Ctx(x: 0)
    }
}

/// Returns the full HTML page with htmx and initial counter.
func pageHtml(count: Int64) -> String {
    var t = Template();
    t.setRaw("counter", counterHtml(count));
    t.render("<!DOCTYPE html><html><head>"
        + "<title>Kestrel Counter</title>"
        + "<script src=\"https://unpkg.com/htmx.org@1.9.10\"></script>"
        + "<style>"
        + "body{{font-family:system-ui,sans-serif;max-width:400px;margin:80px auto;text-align:center;background:#1a1a2e;color:#eee}}"
        + "h1{{color:#e94560}}"
        + "#count{{font-size:6rem;font-weight:bold;margin:20px 0;color:#0f3460;background:#eee;border-radius:16px;padding:20px}}"
        + "button{{font-size:1.5rem;padding:12px 28px;margin:8px;border:none;border-radius:8px;cursor:pointer;font-weight:bold}}"
        + ".inc{{background:#0f3460;color:#eee}}.dec{{background:#e94560;color:#eee}}.rst{{background:#533483;color:#eee}}"
        + "button:hover{{opacity:0.85}}"
        + "</style></head><body>"
        + "<h1>Kestrel Counter</h1>"
        + "<div id=\"counter\">{counter}</div></body></html>")
}

/// Returns the htmx fragment for the counter display and buttons.
func counterHtml(count: Int64) -> String {
    var t = Template();
    t.setRaw("c", count.format());
    t.render("<div id=\"count\">{c}</div><div>"
        + "<button class=\"dec\" hx-post=\"/dec?n={c}\" hx-target=\"#counter\" hx-swap=\"innerHTML\">- 1</button>"
        + "<button class=\"rst\" hx-post=\"/reset\" hx-target=\"#counter\" hx-swap=\"innerHTML\">Reset</button>"
        + "<button class=\"inc\" hx-post=\"/inc?n={c}\" hx-target=\"#counter\" hx-swap=\"innerHTML\">+ 1</button>"
        + "</div>")
}

/// Parses an integer from a string, returning 0 on failure.
func parseInt(s: String) -> Int64 {
    let len = s.byteCount;
    if len == 0 { return 0 }

    var i: Int64 = 0;
    var neg = false;
    if s.byteAtUnchecked(0) == 45 {
        neg = true;
        i = 1
    }
    if i >= len { return 0 }

    var result: Int64 = 0;
    while i < len {
        let b = Int64(from: s.byteAtUnchecked(i));
        if b < 48 or b > 57 { return 0 }
        result = result * 10 + (b - 48);
        i = i + 1
    }

    if neg { 0 - result } else { result }
}

/// Gets the "n" query param as Int64.
func getN(request: Request) -> Int64 {
    match request.query("n") {
        .Some(val) => parseInt(val),
        .None => 0
    }
}

func main() {
    var app = App[Ctx](Ctx(x: 0));

    app.onGet("/", { (req: Request, ctx: Ctx) in
        Response.ok(html: pageHtml(0))
    });

    app.onPost("/inc", { (req: Request, ctx: Ctx) in
        let count = getN(req) + 1;
        Response.ok(html: counterHtml(count))
    });

    app.onPost("/dec", { (req: Request, ctx: Ctx) in
        let count = getN(req) - 1;
        Response.ok(html: counterHtml(count))
    });

    app.onPost("/reset", { (req: Request, ctx: Ctx) in
        Response.ok(html: counterHtml(0))
    });

    let port: UInt16 = 8080;
    let _ = println("Starting counter on http://localhost:8080");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => {
            let _ = println("Error: " + e.description());
        }
    }
}
