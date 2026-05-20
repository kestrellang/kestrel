module notes.handlers

import perch.request.(Request)
import perch.response.(Response)
import perch.json_body.(JsonBody)
import talon.sqlite.database.(Database)
import notes.context.(AppCtx)
import notes.helpers.(errorJson, parseBody, requireUserId, requireIdParam, paginatedJson, parsePagination, currentTimestamp)
import notes.requests.(CreateFolderRequest, UpdateFolderRequest)
import notes.models.(Folder)
import notes.db.(listFolders, countFolders, findFolderById, createFolder, updateFolder, deleteFolder)

public func handleListFolders(req: Request, ctx: AppCtx) -> Response {
    guard let .Some(userId) = requireUserId(req.store) else { return Response.unauthorized() }
    guard let .Ok(db) = Database(ctx.dbPath) else { return Response.internalServerError() }

    let (page, perPage) = parsePagination(req.query("page"), req.query("per_page"));
    let offset = (page - 1) * perPage;

    guard let .Ok(total) = countFolders(db, userId) else { return Response.internalServerError() }
    guard let .Ok(folders) = listFolders(db, userId, perPage, offset) else { return Response.internalServerError() }
    guard let .Ok(data) = folders.toValue() else { return Response.internalServerError() }
    Response.ok(JsonBody(fromRaw: paginatedJson(data, page, perPage, total)))
}

public func handleCreateFolder(req: Request, ctx: AppCtx) -> Response {
    guard let .Some(userId) = requireUserId(req.store) else { return Response.unauthorized() }
    let body = match parseBody[CreateFolderRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    guard let .Ok(db) = Database(ctx.dbPath) else { return Response.internalServerError() }

    let now = currentTimestamp();
    guard let .Ok(folder) = createFolder(db, body.name, userId, now) else {
        return Response.internalServerError()
    }
    guard let .Ok(json) = JsonBody(folder) else { return Response.internalServerError() }
    Response.created(json)
}

public func handleGetFolder(req: Request, ctx: AppCtx) -> Response {
    guard let .Some(userId) = requireUserId(req.store) else { return Response.unauthorized() }
    guard let .Some(folderId) = requireIdParam(req.param("id")) else {
        return Response.badRequest(JsonBody(fromRaw: errorJson("Invalid folder id")))
    }
    guard let .Ok(db) = Database(ctx.dbPath) else { return Response.internalServerError() }

    guard let .Ok(maybeFolder) = findFolderById(db, id: folderId, userId: userId) else {
        return Response.internalServerError()
    }
    guard let .Some(folder) = maybeFolder else { return Response.notFound() }
    guard let .Ok(json) = JsonBody(folder) else { return Response.internalServerError() }
    Response.ok(json)
}

public func handleUpdateFolder(req: Request, ctx: AppCtx) -> Response {
    guard let .Some(userId) = requireUserId(req.store) else { return Response.unauthorized() }
    guard let .Some(folderId) = requireIdParam(req.param("id")) else {
        return Response.badRequest(JsonBody(fromRaw: errorJson("Invalid folder id")))
    }
    let body = match parseBody[UpdateFolderRequest](req.body) {
        .Ok(b) => b,
        .Err(resp) => return resp
    };
    guard let .Ok(db) = Database(ctx.dbPath) else { return Response.internalServerError() }

    let now = currentTimestamp();
    guard let .Ok(maybeFolder) = updateFolder(db, id: folderId, body.name, userId, now) else {
        return Response.internalServerError()
    }
    guard let .Some(folder) = maybeFolder else { return Response.notFound() }
    guard let .Ok(json) = JsonBody(folder) else { return Response.internalServerError() }
    Response.ok(json)
}

public func handleDeleteFolder(req: Request, ctx: AppCtx) -> Response {
    guard let .Some(userId) = requireUserId(req.store) else { return Response.unauthorized() }
    guard let .Some(folderId) = requireIdParam(req.param("id")) else {
        return Response.badRequest(JsonBody(fromRaw: errorJson("Invalid folder id")))
    }
    guard let .Ok(db) = Database(ctx.dbPath) else { return Response.internalServerError() }

    guard let .Ok(_) = deleteFolder(db, id: folderId, userId: userId) else {
        return Response.internalServerError()
    }
    Response.noContent()
}
