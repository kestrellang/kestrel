module notes.handlers

import perch.(HttpError, Json, Path, Query, Created, NoContent)
import datetime.(Instant)
import notes.context.(AuthCtx)
import notes.requests.(CreateFolderRequest, UpdateFolderRequest, Pagination)
import notes.models.(Folder, Page)
import notes.db.(listFolders, countFolders, findFolderById, createFolder, updateFolder, deleteFolder)

// Handlers for /folders. Same shape as `Notes`.
public struct Folders {
    public static func list(ctx: AuthCtx, Query { value: page }: Query[Pagination]) -> Json[Page[Folder]] throws HttpError {
        let total = try countFolders(ctx.db, ctx.userId);
        let folders = try listFolders(ctx.db, ctx.userId, page.perPage, page.offset);
        Json(Page(folders, page, total))
    }

    public static func create(ctx: AuthCtx, Json { value: new }: Json[CreateFolderRequest]) -> Created[Json[Folder]] throws HttpError {
        let folder = try createFolder(ctx.db, new.name, ctx.userId, Instant.now().formatted());
        Created(Json(folder))
    }

    public static func read(ctx: AuthCtx, Path { value: id }: Path[Int64]) -> Json[Folder] throws HttpError {
        guard let some folder = try findFolderById(ctx.db, id: id, userId: ctx.userId) else {
            throw HttpError.notFound()
        }
        Json(folder)
    }

    public static func update(ctx: AuthCtx, (Path { value: id }, Json { value: edit }): (Path[Int64], Json[UpdateFolderRequest])) -> Json[Folder] throws HttpError {
        guard let some folder = try updateFolder(ctx.db, id: id, edit.name, ctx.userId, Instant.now().formatted()) else {
            throw HttpError.notFound()
        }
        Json(folder)
    }

    public static func delete(ctx: AuthCtx, Path { value: id }: Path[Int64]) -> NoContent throws HttpError {
        try deleteFolder(ctx.db, id: id, userId: ctx.userId);
        NoContent()
    }
}
