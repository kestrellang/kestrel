/// Row access and the FromRow protocol for mapping query results to types.

module talon.sqlite.row

import talon.sqlite.value.(SqliteValue, FromSqliteValue)
import talon.sqlite.error.(SqliteError)

/// A single result row from a SQLite query.
///
/// Contains an owned copy of all column values — safe to store and
/// inspect after the query completes.
public struct Row: Cloneable {
    var columns: Array[SqliteValue]

    public init(columns columns: Array[SqliteValue]) {
        self.columns = columns;
    }

    /// Number of columns in this row.
    public var columnCount: Int64 { self.columns.count }

    /// Reads column at `index` as type `T`.
    ///
    /// Use non-optional types for required columns and optional types
    /// for nullable columns:
    /// ```
    /// let id = try row.read[Int64](at: 0);        // throws if NULL
    /// let email = try row.read[String?](at: 2);   // NULL becomes .None
    /// ```
    public func read[T](at index: Int64) -> T throws SqliteError where T: FromSqliteValue {
        guard index < self.columns.count else {
            throw SqliteError.Error("column index \(index) out of bounds (row has \(self.columns.count) columns)");
        }
        try T.fromSqliteValue(self.columns(index))
    }

    /// Returns the raw SqliteValue at `index`.
    public func value(at index: Int64) -> SqliteValue {
        self.columns(index)
    }

    public func clone() -> Row {
        Row(columns: self.columns.clone())
    }
}

/// Types that can be constructed from a database row.
///
/// ```
/// struct User: FromRow {
///     var id: Int64
///     var name: String
///     var email: String?
///
///     static func fromRow(row: Row) -> User throws SqliteError {
///         User(
///             id: try row.read[Int64](at: 0),
///             name: try row.read[String](at: 1),
///             email: try row.read[String?](at: 2)
///         )
///     }
/// }
/// ```
public protocol FromRow {
    static func fromRow(row: Row) -> Self throws SqliteError
}
