module notes.handlers

import perch.(HttpError, Json, Path, Query, Created, NoContent)
import datetime.(Instant)
import notes.context.(AuthCtx)
import notes.requests.(CreateNoteRequest, UpdateNoteRequest, MoveFolderRequest, Pagination)
import notes.models.(Note, Page)
import notes.db.(listNotes, countNotes, findNoteById, createNote, updateNote, deleteNote, moveNoteToFolder)

// Handlers for /notes. Every handler is `(ctx, input) -> R throws HttpError`:
// - `ctx` is whatever the group's middleware produced (AuthCtx here, so the
//   user is already authenticated — no userId parsing in the handler).
// - `input` conforms to `FromRequest`: a single extractor, a tuple of them,
//   or `()` when the handler needs nothing from the request.
// - `R` conforms to `IntoResponse`; the status code lives in the type.
// A thrown HttpError becomes the response; SqliteError converts to a 500.
public struct Notes {
    public static func list(ctx: AuthCtx, Query { value: page }: Query[Pagination]) -> Json[Page[Note]] throws HttpError {
        let total = try countNotes(ctx.db, ctx.userId);
        let notes = try listNotes(ctx.db, ctx.userId, page.perPage, page.offset);
        Json(Page(notes, page, total))
    }

    public static func create(ctx: AuthCtx, Json { value: new }: Json[CreateNoteRequest]) -> Created[Json[Note]] throws HttpError {
        let note = try createNote(ctx.db, new.title, new.body, new.folderId, ctx.userId, Instant.now().formatted());
        Created(Json(note))
    }

    public static func read(ctx: AuthCtx, Path { value: id }: Path[Int64]) -> Json[Note] throws HttpError {
        guard let some note = try findNoteById(ctx.db, id: id, userId: ctx.userId) else {
            throw HttpError.notFound()
        }
        Json(note)
    }

    public static func update(ctx: AuthCtx, (Path { value: id }, Json { value: edit }): (Path[Int64], Json[UpdateNoteRequest])) -> Json[Note] throws HttpError {
        guard let some note = try updateNote(ctx.db, id: id, edit.title, edit.body, ctx.userId, Instant.now().formatted()) else {
            throw HttpError.notFound()
        }
        Json(note)
    }

    public static func delete(ctx: AuthCtx, Path { value: id }: Path[Int64]) -> NoContent throws HttpError {
        try deleteNote(ctx.db, id: id, userId: ctx.userId);
        NoContent()
    }

    public static func move(ctx: AuthCtx, (Path { value: id }, Json { value: move }): (Path[Int64], Json[MoveFolderRequest])) -> Json[Note] throws HttpError {
        guard let some note = try moveNoteToFolder(ctx.db, id: id, move.folderId, ctx.userId, Instant.now().formatted()) else {
            throw HttpError.notFound()
        }
        Json(note)
    }
}
