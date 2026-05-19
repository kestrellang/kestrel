module notes.models

import talon.sqlite.row.(Row, FromRow)
import talon.sqlite.error.(SqliteError)
import quill.value.(Value)
import quill.serialize.(Serialize)
import quill.error.(SerializeError)

public struct Note: FromRow, Serialize, Cloneable {
    public var id: Int64
    public var title: String
    public var body: String
    public var folderId: Int64?
    public var userId: Int64
    public var createdAt: String
    public var updatedAt: String

    public static func fromRow(row: Row) -> Note throws SqliteError {
        Note(
            id: try row.read[Int64](at: 0),
            title: try row.read[String](at: 1),
            body: try row.read[String](at: 2),
            folderId: try row.read[Int64?](at: 3),
            userId: try row.read[Int64](at: 4),
            createdAt: try row.read[String](at: 5),
            updatedAt: try row.read[String](at: 6)
        )
    }

    public func toValue() -> Result[Value, SerializeError] {
        var obj = Dictionary[String, Value]();
        obj.insert("id", Value.Int(self.id));
        obj.insert("title", Value.Str(self.title));
        obj.insert("body", Value.Str(self.body));
        obj.insert("folderId", match self.folderId {
            .Some(id) => Value.Int(id),
            .None => Value.Null
        });
        obj.insert("userId", Value.Int(self.userId));
        obj.insert("createdAt", Value.Str(self.createdAt));
        obj.insert("updatedAt", Value.Str(self.updatedAt));
        .Ok(Value.Obj(obj))
    }

    public func clone() -> Note {
        Note(
            id: self.id,
            title: self.title.clone(),
            body: self.body.clone(),
            folderId: self.folderId,
            userId: self.userId,
            createdAt: self.createdAt.clone(),
            updatedAt: self.updatedAt.clone()
        )
    }
}

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
