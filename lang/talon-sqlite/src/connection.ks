/// Internal Connection type — wraps a sqlite3* handle.
///
/// Sole consumer of ffi.ks. Provides execute/query/execRaw that
/// Database and Transaction delegate to via the module-level helpers.

module talon.sqlite.connection

import talon.sqlite.ffi
import talon.sqlite.error.(SqliteError)
import talon.sqlite.value.(SqliteValue)
import talon.sqlite.sql.(SQL)
import talon.sqlite.row.(Row, FromRow)
import std.core.Copyable

// SQLITE_TRANSIENT tells sqlite3 to copy bound string data immediately
func SQLITE_TRANSIENT() -> RawPointer {
    RawPointer(address: 18446744073709551615)
}

// Helper: read the error message from a sqlite3 db handle
func errorMessage(db: RawPointer) -> String {
    let msgPtr = ffi.sqlite3_errmsg(db);
    guard not msgPtr.isNull else { return "unknown error" }
    let cstr = CString(raw: msgPtr.cast[UInt8]());
    String(from: cstr)
}

// Helper: bind an array of SqliteValues to a prepared statement
func bindParams(stmt: RawPointer, bindings: Array[SqliteValue]) -> () throws SqliteError {
    var i: Int64 = 0;
    while i < bindings.count {
        let paramIndex = Int32(from: i + 1);
        let bindResult = match bindings(i) {
            .Integer(v) => ffi.sqlite3_bind_int64(stmt, paramIndex, v),
            .Real(v) => ffi.sqlite3_bind_double(stmt, paramIndex, v.raw),
            .Text(v) => {
                let cstr = v.toCString();
                let byteCount = Int32(from: v.byteCount);
                let r = ffi.sqlite3_bind_text(stmt, paramIndex, cstr.raw.asRaw(), byteCount, SQLITE_TRANSIENT());
                cstr.free();
                r
            },
            .Null => ffi.sqlite3_bind_null(stmt, paramIndex)
        };
        if bindResult != ffi.SQLITE_OK() {
            throw SqliteError.Error("failed to bind parameter \(i + 1)");
        }
        i = i + 1;
    }
    .Ok(())
}

// Helper: prepare+bind+step with no result rows
func executeOnDb(db: RawPointer, sql: SQL) -> () throws SqliteError {
    var stmtRaw = RawPointer.nullPointer();
    let ctemplate = sql.template.toCString();
    let prepResult = ffi.sqlite3_prepare_v2(
        db,
        ctemplate.raw.asRaw(),
        Int32(from: -1),
        Pointer(to: stmtRaw).asRaw(),
        RawPointer.nullPointer()
    );
    ctemplate.free();

    if prepResult != ffi.SQLITE_OK() {
        throw SqliteError.Error(errorMessage(db));
    }

    try bindParams(stmtRaw, sql.bindings);

    let stepResult = ffi.sqlite3_step(stmtRaw);
    let _ = ffi.sqlite3_finalize(stmtRaw);

    if stepResult != ffi.SQLITE_DONE() and stepResult != ffi.SQLITE_ROW() {
        throw SqliteError.Error(errorMessage(db));
    }
}

// Helper: prepare+bind+step loop, map rows via FromRow
func queryOnDb[R](db: RawPointer, sql: SQL) -> Array[R] throws SqliteError where R: FromRow {
    var stmtRaw = RawPointer.nullPointer();
    let ctemplate = sql.template.toCString();
    let prepResult = ffi.sqlite3_prepare_v2(
        db,
        ctemplate.raw.asRaw(),
        Int32(from: -1),
        Pointer(to: stmtRaw).asRaw(),
        RawPointer.nullPointer()
    );
    ctemplate.free();

    if prepResult != ffi.SQLITE_OK() {
        throw SqliteError.Error(errorMessage(db));
    }

    try bindParams(stmtRaw, sql.bindings);

    var results = Array[R]();
    loop {
        let stepResult = ffi.sqlite3_step(stmtRaw);
        if stepResult == ffi.SQLITE_ROW() {
            let row = readRow(stmtRaw);
            results.append(try R.fromRow(row));
        } else if stepResult == ffi.SQLITE_DONE() {
            break
        } else {
            let msg = errorMessage(db);
            let _ = ffi.sqlite3_finalize(stmtRaw);
            throw SqliteError.Error(msg);
        }
    }
    let _ = ffi.sqlite3_finalize(stmtRaw);
    results
}

// Helper: run a raw SQL string (for BEGIN/COMMIT/ROLLBACK)
func execRawOnDb(db: RawPointer, sql: String) -> () throws SqliteError {
    let csql = sql.toCString();
    let result = ffi.sqlite3_exec(
        db,
        csql.raw.asRaw(),
        RawPointer.nullPointer(),
        RawPointer.nullPointer(),
        RawPointer.nullPointer()
    );
    csql.free();

    if result != ffi.SQLITE_OK() {
        throw SqliteError.Error(errorMessage(db));
    }
    .Ok(())
}

// Helper: read all columns from the current row into an owned Row
func readRow(stmt: RawPointer) -> Row {
    let colCount = Int64(from: ffi.sqlite3_column_count(stmt));
    var columns = Array[SqliteValue]();
    var i: Int64 = 0;
    while i < colCount {
        let col = Int32(from: i);
        let colType = ffi.sqlite3_column_type(stmt, col);
        if colType == ffi.SQLITE_INTEGER() {
            columns.append(SqliteValue.Integer(ffi.sqlite3_column_int64(stmt, col)));
        } else if colType == ffi.SQLITE_FLOAT() {
            columns.append(SqliteValue.Real(Float64(raw: ffi.sqlite3_column_double(stmt, col))));
        } else if colType == ffi.SQLITE_TEXT() {
            let textPtr = ffi.sqlite3_column_text(stmt, col);
            if textPtr.isNull {
                columns.append(SqliteValue.Text(""));
            } else {
                let cstr = CString(raw: textPtr.cast[UInt8]());
                columns.append(SqliteValue.Text(String(from: cstr)));
            }
        } else {
            columns.append(SqliteValue.Null);
        }
        i = i + 1;
    }
    Row(columns: columns)
}

/// Internal connection wrapping a sqlite3 handle.
struct Connection: not Copyable {
    var db: RawPointer

    static func open(path: String) -> Result[Connection, SqliteError] {
        var dbRaw = RawPointer.nullPointer();
        let cpath = path.toCString();
        let result = ffi.sqlite3_open(cpath.raw.asRaw(), Pointer(to: dbRaw).asRaw());
        cpath.free();

        if result != ffi.SQLITE_OK() {
            if not dbRaw.isNull {
                let _ = ffi.sqlite3_close(dbRaw);
            }
            return .Err(SqliteError.Error("failed to open database: " + path));
        }
        .Ok(Connection(db: dbRaw))
    }

    func execute(sql: SQL) -> () throws SqliteError {
        executeOnDb(self.db, sql)
    }

    func query[R](sql: SQL) -> Array[R] throws SqliteError where R: FromRow {
        queryOnDb[R](self.db, sql)
    }

    func execRaw(sql: String) -> () throws SqliteError {
        execRawOnDb(self.db, sql)
    }

    deinit {
        if not self.db.isNull {
            let _ = ffi.sqlite3_close(self.db);
        }
    }
}
