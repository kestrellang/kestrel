module notes.handlers

import perch.request.(Request)
import perch.response.(Response)
import perch.json_body.(JsonBody)
import talon.sqlite.database.(Database)
import talon.sqlite.sql.(SQL)
import notes.context.(AppCtx)
import notes.errors.(errorJson, parseBody, requireUserId, requireIdParam, paginatedJson, parsePagination)
import notes.requests.(CreateFolderRequest, UpdateFolderRequest)
import notes.models.(Folder)
import notes.db.(listFolders, countFolders, findFolderById, createFolder, updateFolder, deleteFolder)

public func handleListFolders(req: Request, ctx: AppCtx) -> Response {
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

    let total = match countFolders(db, userId) {
        .Ok(n) => n,
        .Err(_) => return Response.internalServerError()
    };
    let folders = match listFolders(db, userId, perPage, offset) {
        .Ok(f) => f,
        .Err(_) => return Response.internalServerError()
    };

    match folders.toValue() {
        .Ok(data) => Response.ok(JsonBody(fromRaw: paginatedJson(data, page, perPage, total))),
        .Err(_) => Response.internalServerError()
    }
}

public func handleCreateFolder(req: Request, ctx: AppCtx) -> Response {
    let userId = match requireUserId(req.store) {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let body = match parseBody[CreateFolderRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    let now = "2026-01-01T00:00:00Z";
    match createFolder(db, body.name, userId, now) {
        .Ok(folder) => match JsonBody(folder) {
            .Ok(json) => Response.created(json),
            .Err(_) => Response.internalServerError()
        },
        .Err(_) => Response.internalServerError()
    }
}

public func handleGetFolder(req: Request, ctx: AppCtx) -> Response {
    let userId = match requireUserId(req.store) {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let folderId = match requireIdParam(req.param("id"), "folder id") {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    match findFolderById(db, id: folderId, userId: userId) {
        .Ok(.Some(folder)) => match JsonBody(folder) {
            .Ok(json) => Response.ok(json),
            .Err(_) => Response.internalServerError()
        },
        .Ok(.None) => Response.notFound(),
        .Err(_) => Response.internalServerError()
    }
}

public func handleUpdateFolder(req: Request, ctx: AppCtx) -> Response {
    let userId = match requireUserId(req.store) {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let folderId = match requireIdParam(req.param("id"), "folder id") {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let body = match parseBody[UpdateFolderRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    let now = "2026-01-01T00:00:00Z";
    match updateFolder(db, id: folderId, body.name, userId, now) {
        .Ok(.Some(folder)) => match JsonBody(folder) {
            .Ok(json) => Response.ok(json),
            .Err(_) => Response.internalServerError()
        },
        .Ok(.None) => Response.notFound(),
        .Err(_) => Response.internalServerError()
    }
}

public func handleDeleteFolder(req: Request, ctx: AppCtx) -> Response {
    let userId = match requireUserId(req.store) {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let folderId = match requireIdParam(req.param("id"), "folder id") {
        .Ok(id) => id,
        .Err(resp) => return resp
    };
    let db = match Database(ctx.dbPath) {
        .Ok(db) => db,
        .Err(_) => return Response.internalServerError()
    };

    match deleteFolder(db, id: folderId, userId: userId) {
        .Ok(_) => Response.noContent(),
        .Err(_) => Response.internalServerError()
    }
}
