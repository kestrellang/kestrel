module notes.models

import talon.sqlite.row.(Row, FromRow)
import talon.sqlite.error.(SqliteError)

// Row mapper for counting queries: SELECT COUNT(*) FROM ...
public struct CountRow: FromRow, Cloneable {
    public var count: Int64

    public static func fromRow(row: Row) -> CountRow throws SqliteError {
        CountRow(count: try row.read[Int64](at: 0))
    }

    public func clone() -> CountRow {
        CountRow(count: self.count)
    }
}
