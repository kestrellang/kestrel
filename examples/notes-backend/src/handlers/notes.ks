module notes.handlers

import perch.request.(Request)
import perch.response.(Response)
import perch.json_body.(JsonBody)
import talon.sqlite.database.(Database)
import talon.sqlite.sql.(SQL)
import notes.context.(AppCtx)
import notes.errors.(errorJson, parseBody, requireUserId, requireIdParam, paginatedJson, parsePagination)
import notes.requests.(CreateNoteRequest, UpdateNoteRequest, MoveFolderRequest)
import notes.models.(Note)
import notes.db.(listNotes, countNotes, findNoteById, createNote, updateNote, deleteNote, moveNoteToFolder)

public func handleListNotes(req: Request, ctx: AppCtx) -> Response {
    let userId = match requireUserId(req.store) {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    let (page, perPage) = parsePagination(req.query("page"), req.query("per_page"));
    let offset = (page - 1) * perPage;

    let total = match countNotes(db, userId) {
        .Ok(n) => n,
        .Err(_) => return Response.internalServerError()
    };
    let notes = match listNotes(db, userId, perPage, offset) {
        .Ok(n) => n,
        .Err(_) => return Response.internalServerError()
    };

    match notes.toValue() {
        .Ok(data) => Response.ok(JsonBody(fromRaw: paginatedJson(data, page, perPage, total))),
        .Err(_) => Response.internalServerError()
    }
}

public func handleCreateNote(req: Request, ctx: AppCtx) -> Response {
    let userId = match requireUserId(req.store) {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let body = match parseBody[CreateNoteRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    let now = "2026-01-01T00:00:00Z";
    match createNote(db, body.title, body.body, body.folderId, userId, now) {
        .Ok(note) => match JsonBody(note) {
            .Ok(json) => Response.created(json),
            .Err(_) => Response.internalServerError()
        },
        .Err(_) => Response.internalServerError()
    }
}

public func handleGetNote(req: Request, ctx: AppCtx) -> Response {
    let userId = match requireUserId(req.store) {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let noteId = match requireIdParam(req.param("id"), "note id") {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    match findNoteById(db, id: noteId, userId: userId) {
        .Ok(.Some(note)) => match JsonBody(note) {
            .Ok(json) => Response.ok(json),
            .Err(_) => Response.internalServerError()
        },
        .Ok(.None) => Response.notFound(),
        .Err(_) => Response.internalServerError()
    }
}

public func handleUpdateNote(req: Request, ctx: AppCtx) -> Response {
    let userId = match requireUserId(req.store) {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let noteId = match requireIdParam(req.param("id"), "note id") {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let body = match parseBody[UpdateNoteRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    let now = "2026-01-01T00:00:00Z";
    match updateNote(db, id: noteId, body.title, body.body, userId, now) {
        .Ok(.Some(note)) => match JsonBody(note) {
            .Ok(json) => Response.ok(json),
            .Err(_) => Response.internalServerError()
        },
        .Ok(.None) => Response.notFound(),
        .Err(_) => Response.internalServerError()
    }
}

public func handleDeleteNote(req: Request, ctx: AppCtx) -> Response {
    let userId = match requireUserId(req.store) {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let noteId = match requireIdParam(req.param("id"), "note id") {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    match deleteNote(db, id: noteId, userId: userId) {
        .Ok(_) => Response.noContent(),
        .Err(_) => Response.internalServerError()
    }
}

public func handleMoveNote(req: Request, ctx: AppCtx) -> Response {
    let userId = match requireUserId(req.store) {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let noteId = match requireIdParam(req.param("id"), "note id") {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let body = match parseBody[MoveFolderRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    let now = "2026-01-01T00:00:00Z";
    match moveNoteToFolder(db, id: noteId, body.folderId, userId, now) {
        .Ok(.Some(note)) => match JsonBody(note) {
            .Ok(json) => Response.ok(json),
            .Err(_) => Response.internalServerError()
        },
        .Ok(.None) => Response.notFound(),
        .Err(_) => Response.internalServerError()
    }
}
