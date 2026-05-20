module wall.handlers

import http.status.(StatusCode)
import http.headers.(Headers)
import http.content.(Text)
import perch.request.(Request)
import perch.response.(Response)
import wall.context.(AppCtx)
import wall.render.(wallCss, wallJs)

public func handleHealth(req: Request, ctx: AppCtx) -> Response {
    Response.ok(Text("ok"))
}

public func handleFavicon(req: Request, ctx: AppCtx) -> Response {
    let svg = #"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><text y="28" font-size="28">🪶</text></svg>"#;
    var hdrs = Headers();
    hdrs.setValue("Content-Type", "image/svg+xml");
    hdrs.setValue("Cache-Control", "public, max-age=604800");
    Response(StatusCode.ok(), hdrs, svg)
}

public func handleStylesheet(req: Request, ctx: AppCtx) -> Response {
    var hdrs = Headers();
    hdrs.setValue("Content-Type", "text/css; charset=utf-8");
    hdrs.setValue("Cache-Control", "public, max-age=86400");
    Response(StatusCode.ok(), hdrs, wallCss())
}

public func handleScript(req: Request, ctx: AppCtx) -> Response {
    var hdrs = Headers();
    hdrs.setValue("Content-Type", "application/javascript; charset=utf-8");
    hdrs.setValue("Cache-Control", "public, max-age=86400");
    Response(StatusCode.ok(), hdrs, wallJs())
}
