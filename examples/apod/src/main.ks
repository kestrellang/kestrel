// NASA Astronomy Picture of the Day
//
// A single-page viewer for NASA's APOD feed. The optional `date` query
// parameter (YYYY-MM-DD) selects a specific day; absent, NASA returns today's.
// Uses the public DEMO_KEY API key — fine for a demo, rate-limited per IP.

module apod.main

import perch.(App, Logger, Get, HttpError, Query)
import http.content.(Html)
import swoop.swoop.(Swoop)
import quill.value.(Value)
import quill.deserialize.(Deserialize)
import quill.error.(DeserializeError)
import quill.json.parser.(parseJson)
import apod.ui.(pageHtml, errorPageHtml)
import apod.util.(isIsoDate)

// ============================================================================
// CONTEXT
// ============================================================================

struct Ctx: Cloneable {
    var apiBase: String
    var apiKey: String

    func clone() -> Ctx {
        Ctx(apiBase: self.apiBase.clone(), apiKey: self.apiKey.clone())
    }
}

// ============================================================================
// ROUTES
// ============================================================================

/// The optional `?date=` parameter. We only forward it to NASA if it parses
/// as a valid ISO date — otherwise the API returns a 400 that's not worth
/// surfacing — so anything else reads as "" (today).
struct DateQuery: Deserialize {
    var date: String

    static func fromValue(value: Value) -> DateQuery throws DeserializeError {
        guard let some .Str(raw) = value.value(for: "date") else { return DateQuery(date: "") }
        DateQuery(date: if isIsoDate(raw) { raw } else { "" })
    }
}

struct Handlers {
    /// The picture for `?date=` (today when absent). A failed fetch or an
    /// unparsable reply is still a page — an error page, not an HTTP error.
    static func index(ctx: Ctx, Query { value: query }: Query[DateQuery]) -> Html throws HttpError {
        let date = query.date;
        var url = String();
        url.append(ctx.apiBase);
        url.append("/planetary/apod?api_key=");
        url.append(ctx.apiKey);
        if date.bytes.count > 0 {
            url.append("&date=");
            url.append(date)
        };

        let res = match Swoop().fetch(url) {
            .Ok(res) => res,
            .Err(_) => return Html(errorPageHtml("Could not reach NASA. Check your connection or try again later.", date))
        };
        match parseJson(res.body) {
            .Ok(json) => Html(pageHtml(json, date)),
            .Err(_) => Html(errorPageHtml("Could not parse the response from NASA.", date))
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

@main
func main() {
    // DEMO_KEY is rate-limited (30/hr, 50/day). For heavier use, register at
    // https://api.nasa.gov and pass your own key here.
    let ctx = Ctx(apiBase: "https://api.nasa.gov", apiKey: "DEMO_KEY");
    let app = App(ctx).using(Logger());
    app.on(Get("/"), Handlers.index);

    let port: UInt16 = 8095;
    println("Starting APOD viewer on http://localhost:8095");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => {
            println("Error: " + e.description());
        }
    }
}
