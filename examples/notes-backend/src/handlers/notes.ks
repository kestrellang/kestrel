module notes.handlers

import perch.request.(Request)
import perch.response.(Response)
import perch.json_body.(JsonBody)
import talon.sqlite.database.(Database)
import notes.context.(AppCtx)
import notes.helpers.(errorJson, parseBody, requireUserId, requireIdParam, paginatedJson, parsePagination, currentTimestamp)
import notes.requests.(CreateNoteRequest, UpdateNoteRequest, MoveFolderRequest)
import notes.models.(Note)
import notes.db.(listNotes, countNotes, findNoteById, createNote, updateNote, deleteNote, moveNoteToFolder)

public func handleListNotes(req: Request, ctx: AppCtx) -> Response {
    guard let .Some(userId) = requireUserId(req.store) else { return Response.unauthorized() }
    guard let .Ok(db) = Database(ctx.dbPath) else { return Response.internalServerError() }

    let (page, perPage) = parsePagination(req.query("page"), req.query("per_page"));
    let offset = (page - 1) * perPage;

    guard let .Ok(total) = countNotes(db, userId) else { return Response.internalServerError() }
    guard let .Ok(notes) = listNotes(db, userId, perPage, offset) else { return Response.internalServerError() }
    guard let .Ok(data) = notes.toValue() else { return Response.internalServerError() }
    Response.ok(JsonBody(fromRaw: paginatedJson(data, page, perPage, total)))
}

public func handleCreateNote(req: Request, ctx: AppCtx) -> Response {
    guard let .Some(userId) = requireUserId(req.store) else { return Response.unauthorized() }
    let body = match parseBody[CreateNoteRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    guard let .Ok(db) = Database(ctx.dbPath) else { return Response.internalServerError() }

    let now = currentTimestamp();
    guard let .Ok(note) = createNote(db, body.title, body.body, body.folderId, userId, now) else {
        return Response.internalServerError()
    }
    guard let .Ok(json) = JsonBody(note) else { return Response.internalServerError() }
    Response.created(json)
}

public func handleGetNote(req: Request, ctx: AppCtx) -> Response {
    guard let .Some(userId) = requireUserId(req.store) else { return Response.unauthorized() }
    guard let .Some(noteId) = requireIdParam(req.param("id")) else {
        return Response.badRequest(JsonBody(fromRaw: errorJson("Invalid note id")))
    }
    guard let .Ok(db) = Database(ctx.dbPath) else { return Response.internalServerError() }

    guard let .Ok(maybeNote) = findNoteById(db, id: noteId, userId: userId) else {
        return Response.internalServerError()
    }
    guard let .Some(note) = maybeNote else { return Response.notFound() }
    guard let .Ok(json) = JsonBody(note) else { return Response.internalServerError() }
    Response.ok(json)
}

public func handleUpdateNote(req: Request, ctx: AppCtx) -> Response {
    guard let .Some(userId) = requireUserId(req.store) else { return Response.unauthorized() }
    guard let .Some(noteId) = requireIdParam(req.param("id")) else {
        return Response.badRequest(JsonBody(fromRaw: errorJson("Invalid note id")))
    }
    let body = match parseBody[UpdateNoteRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    guard let .Ok(db) = Database(ctx.dbPath) else { return Response.internalServerError() }

    let now = currentTimestamp();
    guard let .Ok(maybeNote) = updateNote(db, id: noteId, body.title, body.body, userId, now) else {
        return Response.internalServerError()
    }
    guard let .Some(note) = maybeNote else { return Response.notFound() }
    guard let .Ok(json) = JsonBody(note) else { return Response.internalServerError() }
    Response.ok(json)
}

public func handleDeleteNote(req: Request, ctx: AppCtx) -> Response {
    guard let .Some(userId) = requireUserId(req.store) else { return Response.unauthorized() }
    guard let .Some(noteId) = requireIdParam(req.param("id")) else {
        return Response.badRequest(JsonBody(fromRaw: errorJson("Invalid note id")))
    }
    guard let .Ok(db) = Database(ctx.dbPath) else { return Response.internalServerError() }

    guard let .Ok(_) = deleteNote(db, id: noteId, userId: userId) else {
        return Response.internalServerError()
    }
    Response.noContent()
}

public func handleMoveNote(req: Request, ctx: AppCtx) -> Response {
    guard let .Some(userId) = requireUserId(req.store) else { return Response.unauthorized() }
    guard let .Some(noteId) = requireIdParam(req.param("id")) else {
        return Response.badRequest(JsonBody(fromRaw: errorJson("Invalid note id")))
    }
    let body = match parseBody[MoveFolderRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    guard let .Ok(db) = Database(ctx.dbPath) else { return Response.internalServerError() }

    let now = currentTimestamp();
    guard let .Ok(maybeNote) = moveNoteToFolder(db, id: noteId, body.folderId, userId, now) else {
        return Response.internalServerError()
    }
    guard let .Some(note) = maybeNote else { return Response.notFound() }
    guard let .Ok(json) = JsonBody(note) else { return Response.internalServerError() }
    Response.ok(json)
}
