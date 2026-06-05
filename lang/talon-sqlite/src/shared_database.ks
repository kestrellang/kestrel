// SharedDatabase — refcounted database connection for shared ownership.
//
// Unlike Database (which owns a Connection with a deinit that closes the handle),
// SharedDatabase uses manual refcounting so it can be Cloneable. The sqlite3 handle
// is closed exactly once, when the last clone drops.

module talon.sqlite.shared_database

import talon.sqlite.ffi
import talon.sqlite.connection.(executeOnDb, queryOnDb, execRawOnDb, lastInsertRowIdOnDb)
import talon.sqlite.transaction.(Transaction)
import talon.sqlite.executor.(SqliteExecutor)
import talon.sqlite.error.(SqliteError)
import talon.sqlite.sql.(SQL)
import talon.sqlite.row.(FromRow)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator)
import std.core.(fatalError)

// Heap-allocated storage: refcount + sqlite3* handle.
struct SharedDbStorage {
    var refCount: Int64
    var db: RawPointer
}

/// A reference-counted SQLite database connection.
///
/// `SharedDatabase` is `Cloneable` — cloning bumps a refcount and shares
/// the underlying `sqlite3*` handle. The handle is closed when the last
/// clone drops.
///
/// Use this instead of `Database` when the connection must live inside a
/// `Cloneable` context (e.g. a Perch `AppCtx`).
public struct SharedDatabase: Cloneable, SqliteExecutor {
    private var ptr: Pointer[SharedDbStorage]

    /// Opens or creates a SQLite database at the given path.
    public init(path: String) throws SqliteError {
        var dbRaw = RawPointer.nullPointer();
        let cpath = path.toCString();
        let result = ffi.sqlite3_open(cpath.raw.asRaw(), Pointer(to: dbRaw).asRaw());
        cpath.free();

        if result != ffi.SQLITE_OK() {
            if not dbRaw.isNull {
                 ffi.sqlite3_close(dbRaw);
            }
            throw SqliteError.Error("failed to open database: " + path);
        }

        let layout = Layout.of[SharedDbStorage]();
        var allocator = SystemAllocator();
        let rawPtr = allocator.allocate(layout);
        if let .Some(p) = rawPtr {
            self.ptr = p.cast[SharedDbStorage]();
            self.ptr.write(SharedDbStorage(refCount: 1, db: dbRaw));
        } else {
             ffi.sqlite3_close(dbRaw);
            fatalError("SharedDatabase allocation failed")
        }
    }

    // Adopts an existing storage pointer (refcount already bumped by clone).
    private init(inner inner: Pointer[SharedDbStorage]) {
        self.ptr = inner;
    }

    public func clone() -> SharedDatabase {
        var storage = self.ptr.read();
        storage.refCount = storage.refCount + 1;
        self.ptr.write(storage);
        SharedDatabase(inner: self.ptr)
    }

    public func execute(sql: SQL) -> () throws SqliteError {
        executeOnDb(self.ptr.read().db, sql)
    }

    public func query[R](sql: SQL) -> Array[R] throws SqliteError where R: FromRow {
        queryOnDb[R](self.ptr.read().db, sql)
    }

    public func lastInsertRowId() -> Int64 {
        lastInsertRowIdOnDb(self.ptr.read().db)
    }

    public func transaction(body: (Transaction) -> () throws SqliteError) -> () throws SqliteError {
        let dbRaw = self.ptr.read().db;
        try execRawOnDb(dbRaw, "BEGIN");
        let tx = Transaction(db: dbRaw);
        match body(tx) {
            .Ok(_) => execRawOnDb(dbRaw, "COMMIT"),
            .Err(e) => {
                 execRawOnDb(dbRaw, "ROLLBACK");
                throw e;
            }
        }
    }

    private func release() {
        var storage = self.ptr.read();
        storage.refCount = storage.refCount - 1;

        if storage.refCount == 0 {
            if not storage.db.isNull {
                 ffi.sqlite3_close(storage.db);
            }
            let layout = Layout.of[SharedDbStorage]();
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
        } else {
            self.ptr.write(storage)
        }
    }

    deinit {
        self.release()
    }
}
