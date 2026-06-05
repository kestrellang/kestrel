module notes.models

import talon.sqlite.row.(Row, FromRow)
import talon.sqlite.error.(SqliteError)
import quill.value.(Value)
import quill.serialize.(Serialize)
import quill.error.(SerializeError)

public struct Folder: FromRow, Serialize, Cloneable {
    public var id: Int64
    public var name: String
    public var userId: Int64
    public var createdAt: String
    public var updatedAt: String

    public static func fromRow(row: Row) -> Folder throws SqliteError {
        Folder(
            id: try row.read[Int64](at: 0),
            name: try row.read[String](at: 1),
            userId: try row.read[Int64](at: 2),
            createdAt: try row.read[String](at: 3),
            updatedAt: try row.read[String](at: 4)
        )
    }

    public func toValue() -> Result[Value, SerializeError] {
        var obj = Dictionary[String, Value]();
        obj.insert("id", Value.Int(self.id));
        obj.insert("name", Value.Str(self.name));
        obj.insert("userId", Value.Int(self.userId));
        obj.insert("createdAt", Value.Str(self.createdAt));
        obj.insert("updatedAt", Value.Str(self.updatedAt));
        .Ok(Value.Obj(obj))
    }

    public func clone() -> Folder {
        Folder(
            id: self.id,
            name: self.name.clone(),
            userId: self.userId,
            createdAt: self.createdAt.clone(),
            updatedAt: self.updatedAt.clone()
        )
    }
}
