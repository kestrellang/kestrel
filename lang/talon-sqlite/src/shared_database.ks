// SharedDatabase — refcounted database connection for shared ownership.
//
// Unlike Database (which owns a Connection with a deinit that closes the handle),
// SharedDatabase uses manual refcounting so it can be Cloneable. The sqlite3 handle
// is closed exactly once, when the last clone drops.

module talon.sqlite.shared_database

import talon.sqlite.ffi
import talon.sqlite.connection.(executeCachedOnDb, queryCachedOnDb, execRawOnDb, lastInsertRowIdOnDb, finalizeCachedStmts)
import talon.sqlite.transaction.(Transaction)
import talon.sqlite.executor.(SqliteExecutor)
import talon.sqlite.error.(SqliteError)
import talon.sqlite.sql.(SQL)
import talon.sqlite.row.(FromRow)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator, RcBox)
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
    // Prepared-statement cache, shared across clones (one sqlite3* handle ⇒ one
    // cache). Its RcBox refcount tracks clones in lockstep with the manual
    // `refCount` in storage, so the last drop finalizes the statements.
    private var cache: RcBox[Dictionary[String, RawPointer]]

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
            self.cache = RcBox(Dictionary[String, RawPointer]());
        } else {
             ffi.sqlite3_close(dbRaw);
            fatalError("SharedDatabase allocation failed")
        }
    }

    // Adopts an existing storage pointer (refcount already bumped by clone)
    // and a clone of the shared statement cache.
    private init(inner inner: Pointer[SharedDbStorage], cache cache: RcBox[Dictionary[String, RawPointer]]) {
        self.ptr = inner;
        self.cache = cache;
    }

    public func clone() -> SharedDatabase {
        var storage = self.ptr.read();
        storage.refCount = storage.refCount + 1;
        self.ptr.write(storage);
        SharedDatabase(inner: self.ptr, cache: self.cache.clone())
    }

    public func execute(sql: SQL) -> () throws SqliteError {
        executeCachedOnDb(self.ptr.read().db, self.cache, sql)
    }

    public func query[R](sql: SQL) -> Array[R] throws SqliteError where R: FromRow {
        queryCachedOnDb[R](self.ptr.read().db, self.cache, sql)
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
            // Statements must be finalized before the handle is closed, while
            // the cache is still alive (its RcBox field drops after this body).
            finalizeCachedStmts(self.cache);
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
