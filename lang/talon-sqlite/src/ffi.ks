/// Raw C bindings for the SQLite3 API.
///
/// Internal to talon-sqlite — only connection.ks should use these.

module talon.sqlite.ffi

// Result codes
func SQLITE_OK() -> Int32 { 0 }
func SQLITE_ERROR() -> Int32 { 1 }
func SQLITE_ROW() -> Int32 { 100 }
func SQLITE_DONE() -> Int32 { 101 }

// Column types
func SQLITE_INTEGER() -> Int32 { 1 }
func SQLITE_FLOAT() -> Int32 { 2 }
func SQLITE_TEXT() -> Int32 { 3 }
func SQLITE_NULL() -> Int32 { 5 }

// Database lifecycle
@extern(.C, mangleName: "sqlite3_open")
func sqlite3_open(filename: RawPointer, ppDb: RawPointer) -> Int32

@extern(.C, mangleName: "sqlite3_close")
func sqlite3_close(db: RawPointer) -> Int32

@extern(.C, mangleName: "sqlite3_errmsg")
func sqlite3_errmsg(db: RawPointer) -> RawPointer

// Simple execution (no result set)
@extern(.C, mangleName: "sqlite3_exec")
func sqlite3_exec(db: RawPointer, sql: RawPointer, callback: RawPointer, callbackArg: RawPointer, errmsg: RawPointer) -> Int32

// Prepared statements
@extern(.C, mangleName: "sqlite3_prepare_v2")
func sqlite3_prepare_v2(db: RawPointer, sql: RawPointer, nByte: Int32, ppStmt: RawPointer, pzTail: RawPointer) -> Int32

@extern(.C, mangleName: "sqlite3_finalize")
func sqlite3_finalize(stmt: RawPointer) -> Int32

@extern(.C, mangleName: "sqlite3_step")
func sqlite3_step(stmt: RawPointer) -> Int32

@extern(.C, mangleName: "sqlite3_reset")
func sqlite3_reset(stmt: RawPointer) -> Int32

// Parameter binding (1-indexed)
@extern(.C, mangleName: "sqlite3_bind_int64")
func sqlite3_bind_int64(stmt: RawPointer, index: Int32, value: Int64) -> Int32

@extern(.C, mangleName: "sqlite3_bind_double")
func sqlite3_bind_double(stmt: RawPointer, index: Int32, value: lang.f64) -> Int32

@extern(.C, mangleName: "sqlite3_bind_text")
func sqlite3_bind_text(stmt: RawPointer, index: Int32, value: RawPointer, nByte: Int32, destructor: RawPointer) -> Int32

@extern(.C, mangleName: "sqlite3_bind_null")
func sqlite3_bind_null(stmt: RawPointer, index: Int32) -> Int32

// Column access (0-indexed)
@extern(.C, mangleName: "sqlite3_column_count")
func sqlite3_column_count(stmt: RawPointer) -> Int32

@extern(.C, mangleName: "sqlite3_column_type")
func sqlite3_column_type(stmt: RawPointer, col: Int32) -> Int32

@extern(.C, mangleName: "sqlite3_column_int64")
func sqlite3_column_int64(stmt: RawPointer, col: Int32) -> Int64

@extern(.C, mangleName: "sqlite3_column_double")
func sqlite3_column_double(stmt: RawPointer, col: Int32) -> lang.f64

@extern(.C, mangleName: "sqlite3_column_text")
func sqlite3_column_text(stmt: RawPointer, col: Int32) -> RawPointer

@extern(.C, mangleName: "sqlite3_column_bytes")
func sqlite3_column_bytes(stmt: RawPointer, col: Int32) -> Int32
