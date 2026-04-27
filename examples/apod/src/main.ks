// NASA Astronomy Picture of the Day
//
// A single-page viewer for NASA's APOD feed. The optional `date` query
// parameter (YYYY-MM-DD) selects a specific day; absent, NASA returns today's.
// Uses the public DEMO_KEY API key — fine for a demo, rate-limited per IP.

module apod.main

import perch.app.(App)
import perch.request.(Request)
import perch.response.(Response)
import perch.middleware.(logger)
import swoop.swoop.(Swoop)
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

func handleIndex(req: Request, ctx: Ctx) -> Response {
    // `date` is optional. We only forward it to NASA if it parses as a valid
    // ISO date — otherwise the API returns a 400 that's not worth surfacing.
    let date = match req.query("date") {
        .Some(v) => if isIsoDate(v) { v } else { "" },
        .None => ""
    };

    var url = String();
    url.append(ctx.apiBase);
    url.append("/planetary/apod?api_key=");
    url.append(ctx.apiKey);
    if date.byteCount > 0 {
        url.append("&date=");
        url.append(date)
    };

    match Swoop().fetch(url) {
        .Ok(res) => {
            match parseJson(res.body) {
                .Ok(json) => Response.ok(html: pageHtml(json, date)),
                .Err(e) => Response.ok(html: errorPageHtml("Could not parse the response from NASA.", date))
            }
        },
        .Err(e) => {
            Response.ok(html: errorPageHtml("Could not reach NASA. Check your connection or try again later.", date))
        }
    }
}

// ============================================================================
// MAIN
// ============================================================================

func main() {
    // DEMO_KEY is rate-limited (30/hr, 50/day). For heavier use, register at
    // https://api.nasa.gov and pass your own key here.
    let ctx = Ctx(apiBase: "https://api.nasa.gov", apiKey: "DEMO_KEY");
    var app = App[Ctx](ctx);
    app.use(logger[Ctx]());

    app.onGet("/", { (req: Request, ctx: Ctx) in
        handleIndex(req, ctx)
    });

    let port: UInt16 = 8095;
    let _ = println("Starting APOD viewer on http://localhost:8095");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => {
            let _ = println("Error: " + e.description());
        }
    }
}
