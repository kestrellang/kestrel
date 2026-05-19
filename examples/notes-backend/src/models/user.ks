module notes.models

import talon.sqlite.row.(Row, FromRow)
import talon.sqlite.error.(SqliteError)
import quill.value.(Value)
import quill.serialize.(Serialize)
import quill.error.(SerializeError)

public struct User: FromRow, Serialize, Cloneable {
    public var id: Int64
    public var firstName: String
    public var lastName: String
    public var email: String
    public var createdAt: String

    public static func fromRow(row: Row) -> User throws SqliteError {
        User(
            id: try row.read[Int64](at: 0),
            firstName: try row.read[String](at: 1),
            lastName: try row.read[String](at: 2),
            email: try row.read[String](at: 3),
            createdAt: try row.read[String](at: 4)
        )
    }

    public func toValue() -> Result[Value, SerializeError] {
        var obj = Dictionary[String, Value]();
        obj.insert("id", Value.Int(self.id));
        obj.insert("firstName", Value.Str(self.firstName));
        obj.insert("lastName", Value.Str(self.lastName));
        obj.insert("email", Value.Str(self.email));
        obj.insert("createdAt", Value.Str(self.createdAt));
        .Ok(Value.Obj(obj))
    }

    public func clone() -> User {
        User(
            id: self.id,
            firstName: self.firstName.clone(),
            lastName: self.lastName.clone(),
            email: self.email.clone(),
            createdAt: self.createdAt.clone()
        )
    }
}

public struct AuthToken: FromRow, Serialize, Cloneable {
    public var token: String
    public var userId: Int64

    public static func fromRow(row: Row) -> AuthToken throws SqliteError {
        AuthToken(
            token: try row.read[String](at: 0),
            userId: try row.read[Int64](at: 1)
        )
    }

    public func toValue() -> Result[Value, SerializeError] {
        var obj = Dictionary[String, Value]();
        obj.insert("token", Value.Str(self.token));
        .Ok(Value.Obj(obj))
    }

    public func clone() -> AuthToken {
        AuthToken(
            token: self.token.clone(),
            userId: self.userId
        )
    }
}
