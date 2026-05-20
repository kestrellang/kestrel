module wall.main

import std.memory.(RcBox)
import std.os.(getenv)
import perch.app.(App)
import perch.middleware.(Logger)
import talon.sqlite.shared_database.(SharedDatabase)
import wall.context.(AppCtx, SharedState)
import wall.db.(initSchema, loadBlocklist)
import wall.handlers.(handlePage, handlePostNote, handleStylesheet, handleScript, handleFavicon, handleHealth)

func main() {
    match initSchema("wall.db") {
        .Ok(_) => {},
        .Err(e) => {
            println("Failed to initialize database: \(e.description())");
            return
        }
    };

    let db = match SharedDatabase("wall.db") {
        .Ok(d) => d,
        .Err(e) => {
            println("Failed to open database: \(e.description())");
            return
        }
    };

    let state = RcBox(SharedState(
        cachedHtml: String(),
        cacheTimestamp: 0,
        rateLimits: Dictionary[String, Int64](),
        blocklist: match loadBlocklist(db) {
            .Ok(bl) => bl,
            .Err(_) => Set[String]()
        }
    ));

    let ctx = AppCtx(db: db, state: state);
    var app = App[AppCtx](ctx);
    app.use(Logger[AppCtx]());

    app.route(get: "/", handlePage);
    app.route(post: "/api/notes", handlePostNote);
    app.route(get: "/style.css", handleStylesheet);
    app.route(get: "/script.js", handleScript);
    app.route(get: "/favicon.ico", handleFavicon);
    app.route(get: "/health", handleHealth);

    let port: UInt16 = match getenv("PORT") {
        .Some(p) => match UInt16(parsing: p) {
            .Some(n) => n,
            .None => 8080
        },
        .None => 8080
    };
    println("Kestrel Wall listening on http://localhost:\(port)");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => {
            println("Error: \(e.description())");
        }
    }
}
