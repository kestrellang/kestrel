module wall.handlers

import http.status.(StatusCode)
import http.headers.(Headers)
import perch.request.(Request)
import perch.response.(Response)
import wall.context.(AppCtx, SharedState)
import wall.models.(WallNote)
import wall.db.(getRecentNotes)
import wall.render.(renderPage)
import wall.time.(getUnixTime)

public func handlePage(req: Request, ctx: AppCtx) -> Response {
    let now = getUnixTime();
    var state = ctx.state.getValue();

    // Serve from cache if fresh
    if state.cachedHtml.byteCount > 0 and (now - state.cacheTimestamp) < 60 {
        return servePage(state.cachedHtml)
    };

    // Rebuild from DB
    let notes = match getRecentNotes(ctx.db, 50) {
        .Ok(n) => n,
        .Err(_) => Array[WallNote]()
    };
    let html = renderPage(notes);

    state.cachedHtml = html.clone();
    state.cacheTimestamp = now;
    ctx.state.setValue(state);

    servePage(html)
}

func servePage(html: String) -> Response {
    var hdrs = Headers();
    hdrs.setValue("Content-Type", "text/html; charset=utf-8");
    hdrs.setValue("Cache-Control", "public, max-age=60");
    Response(StatusCode.ok(), hdrs, html)
}
