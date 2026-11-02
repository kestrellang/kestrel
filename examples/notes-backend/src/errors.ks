module notes.errors

import perch.(HttpError, IntoHttpError, ServerError)
import talon.sqlite.error.(SqliteError)

// A database failure inside a handler. A UNIQUE violation is the client's
// conflict (409); anything else is a server bug (500). The SQLite message is
// logged, not sent: it names tables and columns.
extend SqliteError: IntoHttpError {
    public consuming func intoHttpError() -> HttpError {
        let message = self.description();
        eprintln(message);
        if message.contains("UNIQUE constraint failed") {
            return HttpError.conflict("Already exists")
        }
        HttpError.serverError()
    }
}

// `try` on a database call in `main` (schema setup, opening the database).
extend Result[T, ServerError]: FromResidual[SqliteError] {
    public static func fromResidual(consuming residual: SqliteError) -> Result[T, ServerError] {
        .Err(ServerError(residual.description()))
    }
}
