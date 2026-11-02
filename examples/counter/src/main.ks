// HTMX Counter Example
//
// A simple web counter with increment, decrement, and reset buttons.
// Uses Perch web framework and htmx for interactive updates.
//
// State is passed via query parameters since Kestrel doesn't yet
// support mutable shared state across requests.

module counter.main

import perch.(App, Logger, Get, Post, HttpError, Query)
import http.content.(Html)
import quill.value.(Value)
import quill.deserialize.(Deserialize)
import quill.error.(DeserializeError)
import plume.(Template)

struct Ctx: Cloneable {
    var x: Int64

    func clone() -> Ctx {
        Ctx(x: self.x)
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
    t.setRaw("c", count.formatted());
    t.render("<div id=\"count\">{c}</div><div>"
        + "<button class=\"dec\" hx-post=\"/dec?n={c}\" hx-target=\"#counter\" hx-swap=\"innerHTML\">- 1</button>"
        + "<button class=\"rst\" hx-post=\"/reset\" hx-target=\"#counter\" hx-swap=\"innerHTML\">Reset</button>"
        + "<button class=\"inc\" hx-post=\"/inc?n={c}\" hx-target=\"#counter\" hx-swap=\"innerHTML\">+ 1</button>"
        + "</div>")
}

/// The `?n=` query parameter: the count the client currently shows.
/// Missing or unparsable is 0.
struct Count: Deserialize {
    var n: Int64

    static func fromValue(value: Value) -> Count throws DeserializeError {
        guard let some .Str(raw) = value.value(for: "n") else { return Count(n: 0) }
        Count(n: Int64(parsing: raw) ?? 0)
    }
}

struct Handlers {
    static func index(ctx: Ctx, input: ()) -> Html throws HttpError = Html(pageHtml(0))

    static func increment(ctx: Ctx, Query { value: count }: Query[Count]) -> Html throws HttpError =
        Html(counterHtml(count.n + 1))

    static func decrement(ctx: Ctx, Query { value: count }: Query[Count]) -> Html throws HttpError =
        Html(counterHtml(count.n - 1))

    static func reset(ctx: Ctx, input: ()) -> Html throws HttpError = Html(counterHtml(0))
}

@main
func main() {
    let app = App(Ctx(x: 0)).using(Logger());
    app.on(Get("/"), Handlers.index);
    app.on(Post("/inc"), Handlers.increment);
    app.on(Post("/dec"), Handlers.decrement);
    app.on(Post("/reset"), Handlers.reset);

    let port: UInt16 = 8080;
    println("Starting counter on http://localhost:8080");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => { println("Error: " + e.description()); }
    }
}
