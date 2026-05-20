module wall.handlers

import http.status.(StatusCode)
import http.headers.(Headers)
import perch.request.(Request)
import perch.response.(Response)
import perch.json_body.(JsonBody)
import quill.value.(Value)
import wall.context.(AppCtx, SharedState)
import wall.db.(createNote, countNotes)
import wall.helpers.(parseForm, formField, getClientIp)
import wall.filter.(containsProfanity)
import wall.time.(getCurrentTimestamp, getUnixTime)

func noteColors() -> Array[String] {
    ["#FEFF9C", "#FF7EB3", "#7AFCFF", "#FFA07A", "#98FB98", "#DDA0DD"]
}

public func handlePostNote(req: Request, ctx: AppCtx) -> Response {
    let fields = parseForm(req.body);
    let username = formField(fields, "username").trimmed().toOwned();
    let message = formField(fields, "message").trimmed().toOwned();

    // Validate lengths
    if username.byteCount == 0 or username.byteCount > 30 {
        return errorResponse("Username must be 1-30 characters")
    };
    if message.byteCount == 0 or message.byteCount > 280 {
        return errorResponse("Message must be 1-280 characters")
    };

    // Read shared state once
    let ip = getClientIp(req);
    let now = getUnixTime();
    var state = ctx.state.getValue();

    // Profanity filter
    if containsProfanity(username, state.blocklist) or containsProfanity(message, state.blocklist) {
        return errorResponse("Please keep it friendly!")
    };

    if let .Some(lastPost) = state.rateLimits(ip) {
        if (now - lastPost) < 30 {
            return Response.tooManyRequests()
        }
    };

    // Pick color based on total count
    let count = match countNotes(ctx.db) {
        .Ok(c) => c,
        .Err(_) => 0
    };
    let colors = noteColors();
    let color = colors(count % 6);

    // Insert note
    let timestamp = getCurrentTimestamp();
    let note = match createNote(ctx.db, username, message, color, timestamp) {
        .Ok(n) => n,
        .Err(_) => return Response.internalServerError()
    };

    // Invalidate page cache and update rate limit
    state.cacheTimestamp = 0;
    state.rateLimits.insert(ip, now);
    ctx.state.setValue(state);

    // Return JSON for optimistic update
    var obj = Dictionary[String, Value]();
    obj.insert("ok", Value.Boolean(true));
    obj.insert("id", Value.Int(note.id));
    obj.insert("username", Value.Str(note.username));
    obj.insert("message", Value.Str(note.message));
    obj.insert("color", Value.Str(note.color));

    var hdrs = Headers();
    hdrs.setValue("Content-Type", "application/json");
    hdrs.setValue("Cache-Control", "no-store");
    let body = JsonBody(fromRaw: Value.Obj(obj));
    Response.created(body).withHeader("Cache-Control", "no-store")
}

func errorResponse(message: String) -> Response {
    var obj = Dictionary[String, Value]();
    obj.insert("error", Value.Str(message));
    Response.badRequest(JsonBody(fromRaw: Value.Obj(obj)))
}
