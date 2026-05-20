module wall.models

import talon.sqlite.row.(Row, FromRow)
import talon.sqlite.error.(SqliteError)

public struct WallNote: FromRow, Cloneable {
    public var id: Int64
    public var username: String
    public var message: String
    public var color: String
    public var createdAt: String

    public static func fromRow(row: Row) -> WallNote throws SqliteError {
        WallNote(
            id: try row.read[Int64](at: 0),
            username: try row.read[String](at: 1),
            message: try row.read[String](at: 2),
            color: try row.read[String](at: 3),
            createdAt: try row.read[String](at: 4)
        )
    }

    public func clone() -> WallNote {
        WallNote(
            id: self.id,
            username: self.username.clone(),
            message: self.message.clone(),
            color: self.color.clone(),
            createdAt: self.createdAt.clone()
        )
    }
}

public struct BlocklistWord: FromRow, Cloneable {
    public var word: String

    public static func fromRow(row: Row) -> BlocklistWord throws SqliteError {
        BlocklistWord(word: try row.read[String](at: 0))
    }

    public func clone() -> BlocklistWord {
        BlocklistWord(word: self.word.clone())
    }
}

public struct CountRow: FromRow, Cloneable {
    public var count: Int64

    public static func fromRow(row: Row) -> CountRow throws SqliteError {
        CountRow(count: try row.read[Int64](at: 0))
    }

    public func clone() -> CountRow {
        CountRow(count: self.count)
    }
}
