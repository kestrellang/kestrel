/// SQLite value types, Bindable protocol for query parameterization,
/// and FromSqliteValue protocol for type-safe column access.

module talon.sqlite.value

import talon.sqlite.error.(SqliteError)

/// A dynamically-typed SQLite value.
public enum SqliteValue: Cloneable {
    case Integer(Int64)
    case Real(Float64)
    case Text(String)
    case Null

    public func clone() -> SqliteValue {
        match self {
            .Integer(v) => SqliteValue.Integer(v),
            .Real(v) => SqliteValue.Real(v),
            .Text(v) => SqliteValue.Text(v.clone()),
            .Null => SqliteValue.Null
        }
    }
}

/// Types that can be bound as SQL query parameters.
///
/// Conforming to `Bindable` allows a type to be interpolated into `SQL`
/// strings. Non-conforming types produce a compile-time error, preventing
/// SQL injection by construction.
public protocol Bindable {
    func toSqliteValue() -> SqliteValue
}

extend Int64: Bindable {
    public func toSqliteValue() -> SqliteValue = .Integer(self)
}

extend Float64: Bindable {
    public func toSqliteValue() -> SqliteValue = .Real(self)
}

extend String: Bindable {
    public func toSqliteValue() -> SqliteValue = .Text(self)
}

/// Types that can be extracted from a SQLite column value.
///
/// Use `Int64`, `String`, `Float64` for required columns (throws on NULL).
/// Use `Int64?`, `String?`, `Float64?` for nullable columns (NULL → `.None`).
///
/// ```
/// let id = try row.get[Int64](at: 0);        // throws if NULL
/// let email = try row.get[String?](at: 2);   // NULL → .None
/// ```
public protocol FromSqliteValue {
    static func fromSqliteValue(value: SqliteValue) -> Self throws SqliteError
}

extend Int64: FromSqliteValue {
    public static func fromSqliteValue(value: SqliteValue) -> Int64 throws SqliteError {
        match value {
            .Integer(v) => v,
            .Null => throw SqliteError.Error("unexpected null for Int64 column"),
            _ => throw SqliteError.Error("expected integer column")
        }
    }
}

extend Float64: FromSqliteValue {
    public static func fromSqliteValue(value: SqliteValue) -> Float64 throws SqliteError {
        match value {
            .Real(v) => v,
            .Null => throw SqliteError.Error("unexpected null for Float64 column"),
            _ => throw SqliteError.Error("expected real column")
        }
    }
}

extend String: FromSqliteValue {
    public static func fromSqliteValue(value: SqliteValue) -> String throws SqliteError {
        match value {
            .Text(v) => v,
            .Null => throw SqliteError.Error("unexpected null for String column"),
            _ => throw SqliteError.Error("expected text column")
        }
    }
}

extend Optional[T]: FromSqliteValue where T: FromSqliteValue {
    public static func fromSqliteValue(value: SqliteValue) -> T? throws SqliteError {
        match value {
            .Null => .None,
            _ => .Some(try T.fromSqliteValue(value))
        }
    }
}
